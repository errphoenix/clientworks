use crate::api::ApiContext;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io, path::Path,
    fs::{
        self, File
    },
    fmt::Display
};

fn save(api: &mut ApiContext) -> Result<(), String> {
    match api.servers.write_to_file(&api.save) {
        Err(e) => {
            warn!("Failed to write server list: {e}");
            Err(e.to_string())
        },
        Ok(_) => Ok(())
    }
}

pub fn create(api: &mut ApiContext, name: String, ip: String, port: u16) -> Result<(), String> {
    if api.servers.0.contains_key(&name) {
        return Err(format!("Server {name} already exists"));
    }
    info!("Creating server {ip}:{port} as {name}");
    api.servers
        .0
        .insert(name.clone(), Server::new(name, ip, port));
    save(api)
}

pub fn delete(api: &mut ApiContext, name: String) -> Result<(), String> {
    if !api.servers.0.contains_key(&name) {
        return Err(format!("Server {name} does not exist"));
    }
    info!("Deleting server {name}");
    api.servers.0.remove(&name);
    save(api)
}

#[derive(Serialize, Deserialize)]
pub struct List(pub(crate) HashMap<String, Server>);

impl List {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn from_file(path: &Path) -> Self {
        let path = path.join("servers.json");
        if !path.exists() {
            fs::write(&path, "{}");
        }
        let raw = fs::read_to_string(&path);
        if let Ok(content) = raw {
            match serde_json::from_str(content.as_str()) {
                Ok(list) => return list,
                Err(e) => error!("Failed to parse server list: {e}"),
            }
        }
        error!("Failed to load server list from {path:?}");
        Self::new()
    }

    pub fn write_to_file(&self, path: &Path) -> io::Result<()> {
        let path = path.join("servers.json");
        info!("Writing server list to {path:?}");
        fs::write(path, serde_json::to_string_pretty(self)?)
    }

    pub fn get_server(&self, name: &String) -> Option<&Server> {
        self.0.get(name)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Server {
    pub name: String,
    pub ip: String,
    pub port: u16,
}

impl Display for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

impl Server {
    pub(crate) fn new(name: String, ip: String, port: u16) -> Self {
        Self { name, ip, port }
    }
}
