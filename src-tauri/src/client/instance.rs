use std::{
    collections::VecDeque, 
    path::PathBuf, fs,
    sync::{
        Arc, Mutex
    }, 
    ops::Deref, 
    fmt::{
        self, Formatter
    }};
use crate::{
    api::Server,
    client, client::{
        AuthProtocol, Version,
        network::ConnectionHandle
    }
};
use azalea::{
    app::PluginGroup,
    Account, ClientBuilder, 
    prelude::*, AccountOpts, 
    protocol::{
        packets::game::ClientboundGamePacket, 
        ServerAddress
    }, FormattedText, DefaultPlugins,
    DefaultBotPlugins
};
use azalea_chat::style::{Ansi, ChatFormatting};
use azalea_viaversion::ViaVersionPlugin;
use tokio::task::{JoinError, JoinHandle};
use uuid::Uuid;

impl From<Server> for ServerAddress {
    fn from(value: Server) -> Self {
        Self {
            host: value.ip,
            port: value.port
        }
    }
}

pub struct Info {
    pub username: String,
    pub uuid: String,
    pub auth: Arc<AuthProtocol>
}

type AzaleaClient = Arc<Mutex<Option<Client>>>;

pub struct ClientInstance {
    pub id: Uuid,
    pub info: Info,
    pub handle: Option<ConnectionHandle>, // TODO currently unused, might be discarded
    pub target: Server,
    pub version: Version,
    pub logs_location: PathBuf,           // TODO implement logging to file
    run_state: Arc<Mutex<bool>>,
    chat_inputs: ChatInputs,
    client: AzaleaClient,                 // TODO figure out a way to store this lol
    account: Account,
    client_thread: Option<JoinHandle<()>>
}

type ChatHistory = Arc<Mutex<Vec<String>>>;
type ChatInputs = Arc<Mutex<VecDeque<String>>>;

#[derive(Default, Clone, Component)]
pub struct ClientState {
    pub instance_key: Uuid,
    pub chat_history: ChatHistory,
    pub chat_inputs: ChatInputs,
    pub run_state: Arc<Mutex<bool>>,
}

fn create_azalea_account(protocol: &AuthProtocol) -> Account {
    match protocol {
        AuthProtocol::Offline(username) => {
            Account {
                username: username.clone(),
                access_token: None,
                // No UUID here. We calculate the UUID for the front-end only, but the
                // actual UUID calculation for offline accounts is destined to the
                // server implementation.
                uuid: None,
                account_opts: AccountOpts::Offline {
                    username: username.clone()
                },
                certs: Arc::new(parking_lot::Mutex::new(None))
            }
        },
        AuthProtocol::Microsoft(token, msa, profile) => {
            Account {
                username: profile.username.clone(),
                access_token: Some(Arc::new(parking_lot::Mutex::new(token.clone()))),
                uuid: Some(profile.uuid),
                account_opts: AccountOpts::MicrosoftWithAccessToken {
                    msa: Arc::new(parking_lot::Mutex::new(*msa.clone()))
                },
                certs: Arc::new(parking_lot::Mutex::new(None))
            }
        }
    }
}

#[allow(unused)]
async fn handle(client: Client, event: Event, state: ClientState) -> anyhow::Result<()> {
    match event {
        Event::Tick => {
            let running = {
                *state.run_state.lock().unwrap()
            };
            if !running {
                {
                    let mut chat = state.chat_history.lock().unwrap();
                    chat.push("Encountered non-running state notification on tick update, disconnecting...".to_owned());
                }
                client.disconnect();
                return Ok(())
            }

            {
                let mut guard = state.chat_inputs.lock().unwrap();
                let count = guard.len();
                for message in guard.iter() {
                    client.chat(message);
                }
                for _ in 0..count {
                    guard.pop_front();
                }
            }

        }
        Event::Chat(msg) => {
            {
                let mut chat = state.chat_history.lock().unwrap();
                chat.push(msg.message().to_ansi());
            }
            client::hooks::chatlog::set_active(state.instance_key, state.chat_history.clone());
        },
        Event::Init => {
            let mut chat = state.chat_history.lock().unwrap();
            let green = Ansi::rgb(ChatFormatting::Green.color().unwrap());
            chat.push(format!("{green}Successfully connected to server."));
            // chat.push("§aRun '.list' for a list of players on the current server.".to_owned());
        }
        Event::Disconnect(reason) => {
            let mut chat = state.chat_history.lock().unwrap();
            let red = Ansi::rgb(ChatFormatting::Red.color().unwrap());
            chat.push(format!("{red}Disconnected from server: {}",
                              reason.unwrap_or(FormattedText::from("No reason provided.")))
            );
        }
        Event::Packet(packet) => {
            let packet = packet.clone();
            match packet.deref() {
                ClientboundGamePacket::Disconnect(packet) => {
                    let mut chat = state.chat_history.lock().unwrap();
                    let red = Ansi::rgb(ChatFormatting::Red.color().unwrap());
                    chat.push(format!("{red}Disconnected from server: {}", packet.reason));
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

impl ClientInstance {
    pub fn new(id: Uuid, username: String, uuid: &Uuid,
               auth: Arc<AuthProtocol>, server: Server,
               version: Option<Version>, logs_location: PathBuf) -> Self {
        Self {
            id,
            account: create_azalea_account(&auth),
            info: Info {
                username,
                uuid: uuid.to_string(),
                auth
            },
            version: version.unwrap_or(Version::V1_21),
            handle: None,
            client: Arc::new(Mutex::new(None)),
            logs_location: logs_location.join(id.to_string()),
            target: server,
            run_state: Arc::new(Mutex::new(false)),
            chat_inputs: Arc::new(Mutex::new(VecDeque::new())),
            client_thread: None

        }
    }

    /// Simply wraps over the running state mutex
    pub fn is_running(&self) -> bool {
        *self.run_state.lock().unwrap()
    }

    /// Appends a chat message input. These are consumed by the client thread every tick
    /// and sent onto the server by the client.
    ///
    /// This also handles the execution of instance commands, such as '.list'
    ///
    /// Does not distinguish between chat and commands.
    pub fn send_message(&mut self, message: String) {
        let mut guard = self.chat_inputs.lock().unwrap();
        guard.push_back(message);
    }

    /// Connect the client to the specified target server.
    /// If the client is currently connected, it will kill the current connection thread before
    /// initiating the requested connection.
    ///
    /// Also, sets the current running state to true.
    pub fn connect(&mut self) {
        self.kill().unwrap_or_default();
        {
            *self.run_state.lock().unwrap() = true;
        }

        let instance_key = self.id;
        let account = self.account.clone();
        let target = self.target.clone();
        let version = self.version.clone();

        let run_state = self.run_state.clone();
        let chat_inputs = self.chat_inputs.clone();

        self.client_thread = Some(tokio::spawn(async move {
            let builder = ClientBuilder::new_without_plugins()
                .add_plugins(DefaultPlugins.build()
                    .disable::<bevy_log::LogPlugin>()
                )
                .add_plugins(DefaultBotPlugins.build())
                .add_plugins(ViaVersionPlugin::start(version.to_string()).await)
                .set_handler(handle);
            builder.set_state(
                ClientState {
                    instance_key,
                    run_state,
                    chat_inputs,
                    ..Default::default()
                }
            )
                .reconnect_after(None)
                .start(account, target)
                .await.unwrap();
        }));
    }

    /// Notifies the client to disconnect, which will then happen on the next tick.
    ///
    /// Alternative for [`Self::disconnect`]
    pub fn disconnect_notify(&mut self) -> Result<(), String> {
        client::hooks::chatlog::remove_active(&self.id);
        {
            if !*self.run_state.lock().unwrap() {
                return Err("Client is not connected [state]".to_owned());
            }
        }
        {
            *self.run_state.lock().unwrap() = false;
        }
        Ok(())
    }

    /// Directly disconnects from Azalea's client handle.
    ///
    /// TODO, use [`Self::disconnect_notify`]
    pub fn disconnect(&mut self) -> Result<(), String> {
        client::hooks::chatlog::remove_active(&self.id);
        let mut guard = self.client.lock().unwrap();
        if let Some(client) = guard.take() {
            client.disconnect();
            if let Some(thread) = self.client_thread.take() {
                thread.abort();
            }
            Ok(())
        } else {
            Err("Client is not connected".to_string())
        }
    }

    /// Directly kills the running client thread, if present.
    /// Once the thread has aborted, the client run state is also notified of this change.
    ///
    /// Use is discouraged unless necessary.
    pub fn kill(&mut self) -> Result<(), String> {
        client::hooks::chatlog::remove_active(&self.id);
        if let Some(handle) = self.client_thread.take() {
            handle.abort();
            {
                *self.run_state.lock().unwrap() = false;
            }
            Ok(())
        } else {
            Err("Client is not connected".to_string())
        }
    }

    /// TODO
    pub fn get_logs(&self) -> String {
        fs::read_to_string(&self.logs_location).unwrap_or_default()
    }
}