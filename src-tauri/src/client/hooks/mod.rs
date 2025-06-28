mod payload;
pub mod chatlog;

use log::{debug, error, info};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use uuid::Uuid;
pub use payload::*;

pub struct Event {
    pub key: Uuid,
    pub payload: Payload
}

pub struct Channel {
    pub sender: mpsc::Sender<Event>,
    pub thread: tokio::task::JoinHandle<()>,
    pub chatlog: Option<tokio::task::JoinHandle<()>>
}

/// Starts a communication thread between the client controllers and the tauri frontend.
/// All events are emitted using the instance UUID as identifier, with a payload containing
/// the event data as JSON, see [`Payload`]
pub fn init(tauri_app: AppHandle) -> Channel {
    let (tx, mut rx) = mpsc::channel::<Event>(32);
    let thread = {
        let handle = tauri_app.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match handle.emit(event.key.to_string().as_str(), event.payload) {
                    Ok(_) => {
                        debug!("Event emitted for: {}", event.key);
                    },
                    Err(err) => {
                        error!("Error emitting event: {err}");
                    }
                }
            }
        })
    };

    info!("Channel hooks thread started");
    Channel {
        sender: tx,
        thread,
        chatlog: None
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        if let Some(chatlog) = &self.chatlog {
            chatlog.abort()
        }
        self.close();
    }
}

impl Channel {
    pub fn close(&mut self) {
        self.thread.abort();
    }

    pub fn init_chatlog(&mut self, tauri_app: AppHandle) {
        if self.chatlog.is_some() {
            return;
        }
        self.chatlog = Some(chatlog::start_thread(tauri_app))
    }

    pub fn send(&mut self, key: Uuid, payload: Payload) {
        let tx = self.sender.clone();
        tokio::spawn(async move {
            match tx.send(Event {
                key, payload
            }).await {
                Ok(_) => {},
                Err(err) => {
                    error!("Error sending event: {err}");
                }
            };
        });
    }
}