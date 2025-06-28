use log::{
    error, info, warn
};
use serde::{
    Deserialize,
    Serialize
};
use std::{
    collections::HashMap,
    io, path::Path,
    fs::{
        self, File
    },
    ops::Deref,
    str::FromStr
};
use azalea::ecs::error::warn;
use uuid::Uuid;
use crate::{
    api::{ApiContext, Server},
    client::{
        auth::MinecraftProfile,
        ClientController,
        Version
    }
};

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientConnection {
    pub id: Uuid,
    pub version: Version,
    pub server: Server
}

impl ClientConnection {
    pub fn new(id: Uuid, version: Version, target: Server) -> Self {
        Self {
            id, version,
            server: target
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Client {
    pub id: Uuid,
    pub username: String,
    pub uuid: Uuid,
    pub auth: AuthType,
    pub connections: HashMap<Uuid, ClientConnection>
}

impl Client {
    pub fn new(id: Uuid, username: String, uuid: Uuid, auth: AuthType) -> Self {
        Self {
            id,
            username,
            uuid,
            auth,
            connections: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AuthType {
    Offline,
    Microsoft,
}

fn save(api: &mut ApiContext) -> Result<(), String> {
    match api.clients.write_to_file(&api.save) {
        Err(e) => {
            warn!("Failed to write client list: {e}");
            Err(e.to_string())
        },
        Ok(_) => Ok(())
    }
}

/// Register a new client from a Minecraft profile.
///
/// # Parameters
/// * `profile` - the [`MinecraftProfile`] to create the account from
///
/// # Errors
/// * `Client already exists` - if the client already exists
/// * `Failed to write client list` - if the client list could not be saved
///
/// # Returns
/// The randomly-generated v4 UUID the new client is bound to
pub fn register(api: &mut ApiContext, profile: &MinecraftProfile) -> Result<Uuid, String> {
    if api.clients.get_by_username(&profile.username).is_some() {
        return Err(format!("Client {} already exists", profile.username));
    }
    info!("Creating client {}", profile.username);
    let id = Uuid::new_v4();
    api.clients.0
        .insert(id, Client::new(id, profile.username.clone(),
                                profile.uuid, {
                                    if profile.authenticated {
                                        AuthType::Microsoft
                                    } else {
                                        AuthType::Offline
                                    }
                                })
        );
    save(api)?;
    Ok(id)
}

pub fn unregister(api: &mut ApiContext, uuid: String) -> Result<(), String> {
    let client_id: Option<Uuid> = {
        let client = api.clients.get_by_mc_uuid(
            &Uuid::from_str(&uuid).unwrap_or_default()
        );
        if let Some(client) = client {
            Some(client.id)
        } else {
            None
        }
    };
    if let Some(id) = client_id {
        info!("Deleting client {uuid}");
        api.clients.0.remove(&id);
        save(api)
    } else {
        Err(format!("Client {uuid} does not exist"))
    }
}

#[derive(Serialize, Deserialize)]
pub struct List(pub HashMap<Uuid, Client>);

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn contains_uuid(&mut self, mc_uuid: &Uuid) -> bool {
        self.0.iter_mut().any(
            |mut e| e.1.uuid == *mc_uuid
        )
    }

    pub fn get_by_username(&self, username: &str) -> Option<&Client> {
        self.0.iter().find_map(
            |e| {
                if e.1.username == username {
                    Some(e.1)
                } else {
                    None
                }
            }
        )
    }

    pub fn get_by_mc_uuid(&mut self, mc_uuid: &Uuid) -> Option<&mut Client> {
        self.0.iter_mut().find_map(
            |mut e| {
                if e.1.uuid == *mc_uuid {
                    Some(e.1)
                } else {
                    None
                }
            }
        )
    }

    pub fn get_by_id(&self, id: &Uuid) -> Option<&Client> {
        self.0.get(id)
    }

    pub fn get_mut_by_id(&mut self, id: &Uuid) -> Option<&mut Client> {
        self.0.get_mut(id)
    }

    pub fn from_file(path: &Path) -> Self {
        let path = path.join("clients.json");
        if !path.exists() {
            fs::write(&path, "{}");
        }
        let raw = fs::read_to_string(&path);
        if let Ok(content) = raw {
            match serde_json::from_str(content.as_str()) {
                Ok(list) => return list,
                Err(e) => error!("Failed to parse client list: {e}"),
            }
        }
        error!("Failed to load client list from {path:?}");
        Self::new()
    }

    pub fn write_to_file(&self, path: &Path) -> io::Result<()> {
        let path = path.join("clients.json");
        info!("Writing client list to {path:?}");
        fs::write(path, serde_json::to_string_pretty(self)?)
    }
}
