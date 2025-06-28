use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::Duration
};
use lazy_static::lazy_static;
use tauri::{AppHandle, Manager};
use uuid::Uuid;
use crate::{
    AppState,
    client::hooks::Payload
};

type ChatHistory = Arc<Mutex<Vec<String>>>;
type ActiveLogs = RwLock<HashMap<Uuid, ChatHistory>>;

lazy_static! {
    static ref ACTIVE_LOGS: ActiveLogs = RwLock::new(HashMap::new());
}

pub fn set_active(uuid: Uuid, chat_history: ChatHistory) {
    let present = {
        let guard = ACTIVE_LOGS.read().unwrap();
        guard.contains_key(&uuid)
    };
    if !present {
        let mut guard = ACTIVE_LOGS.write().unwrap();
        guard.insert(uuid, chat_history.clone());

    }
}

pub fn remove_active(uuid: &Uuid) {
    ACTIVE_LOGS.write().unwrap().remove(&uuid);
}

pub fn start_thread(handle: AppHandle) -> tokio::task::JoinHandle<()> {
    let handle = handle.clone();
    tokio::spawn(async move {
        let state = handle.state::<AppState>();
        loop {
            let active: Vec<_> = {
                let guard = ACTIVE_LOGS.read().unwrap();
                guard.keys().cloned().collect()
            };

            for id in active {
                let log_guard = ACTIVE_LOGS.read().unwrap();
                if let Some(history) = log_guard.get(&id) {
                    let mut history = history.lock().unwrap();
                    while let Some(message) = history.pop() {
                        let mut com_guard = state.com_channel.lock().unwrap();
                        com_guard.send(id, Payload::Chat { message });
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    })
}