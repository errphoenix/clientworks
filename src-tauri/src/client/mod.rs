use uuid::Uuid;
use std::{
    collections::HashMap,
    fmt::Display,
    str::FromStr,
    sync::Arc
};
use std::path::PathBuf;
use azalea_auth::{
    AccessTokenResponse,
    cache::ExpiringValue
};
use lazy_static::lazy_static;
use log::info;
use serde::{Deserialize, Deserializer, Serialize};
use crate::{
    api::{
        auth::MinecraftAuthCache,
        {Server, ApiContext}
    },
    client::auth::MinecraftProfile,
};

pub mod auth;
pub mod network;
mod instance;
pub mod hooks;

#[allow(unused)]
pub use instance::{
    ClientInstance,
    ClientState,
    Info,
    soft_kill
};

lazy_static! {
    static ref LOG_DIR: PathBuf = dirs::data_dir().unwrap_or_default();
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    V1_16,
    V1_16_1,
    V1_16_2,
    V1_16_3,
    V1_16_4,
    V1_16_5,
    V1_17,
    V1_17_1,
    V1_18,
    V1_18_1,
    V1_18_2,
    V1_19,
    V1_19_1,
    V1_19_2,
    V1_20,
    V1_20_1,
    V1_20_2,
    V1_20_3,
    V1_20_4,
    V1_20_5,
    V1_21,
    V1_21_1,
    V1_21_2,
    V1_21_3,
    V1_21_4,
    V1_21_5,
    V1_21_6,
    V1_21_7,
}

impl Default for Version {
    fn default() -> Self {
        Self::V1_21_7
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'a> Deserialize<'a> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>
    {
        let s = String::deserialize(deserializer)?;
        Version::from_string(&s).ok_or_else(|| serde::de::Error::custom("Invalid version"))
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s)
            .ok_or_else(|| format!("Invalid version: {s}"))
    }
}

impl Version {
    pub fn from_string(version: &str) -> Option<Self> {
        match version {
            "1.16" => Some(Self::V1_16),
            "1.16.1" => Some(Self::V1_16_1),
            "1.16.2" => Some(Self::V1_16_2),
            "1.16.3" => Some(Self::V1_16_3),
            "1.16.4" => Some(Self::V1_16_4),
            "1.16.5" => Some(Self::V1_16_5),
            "1.17" => Some(Self::V1_17),
            "1.17.1" => Some(Self::V1_17_1),
            "1.18" => Some(Self::V1_18),
            "1.18.1" => Some(Self::V1_18_1),
            "1.18.2" => Some(Self::V1_18_2),
            "1.19" => Some(Self::V1_19),
            "1.19.1" => Some(Self::V1_19_1),
            "1.19.2" => Some(Self::V1_19_2),
            "1.20" => Some(Self::V1_20),
            "1.20.1" => Some(Self::V1_20_1),
            "1.20.2" => Some(Self::V1_20_2),
            "1.20.3" => Some(Self::V1_20_3),
            "1.20.4" => Some(Self::V1_20_4),
            "1.20.5" => Some(Self::V1_20_5),
            "1.21" => Some(Self::V1_21),
            "1.21.1" => Some(Self::V1_21_1),
            "1.21.2" => Some(Self::V1_21_2),
            "1.21.3" => Some(Self::V1_21_3),
            "1.21.4" => Some(Self::V1_21_4),
            "1.21.5" => Some(Self::V1_21_5),
            "1.21.6" => Some(Self::V1_21_6),
            "1.21.7" => Some(Self::V1_21_7),
            _ => None,
        }
    }

    pub fn all() -> Vec<Version> {
        vec![
            Self::V1_16, Self::V1_16_1, Self::V1_16_2, Self::V1_16_3, Self::V1_16_4, Self::V1_16_5,
            Self::V1_17, Self::V1_17_1,
            Self::V1_18, Self::V1_18_1, Self::V1_18_2,
            Self::V1_19, Self::V1_19_1, Self::V1_19_2,
            Self::V1_20, Self::V1_20_1, Self::V1_20_2, Self::V1_20_3, Self::V1_20_4, Self::V1_20_5,
            Self::V1_21, Self::V1_21_1, Self::V1_21_2, Self::V1_21_3, Self::V1_21_4, Self::V1_21_5, Self::V1_21_6, Self::V1_21_7,
        ]
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::V1_16 => write!(f, "1.16"),
            Version::V1_16_1 => write!(f, "1.16.1"),
            Version::V1_16_2 => write!(f, "1.16.2"),
            Version::V1_16_3 => write!(f, "1.16.3"),
            Version::V1_16_4 => write!(f, "1.16.4"),
            Version::V1_16_5 => write!(f, "1.16.5"),
            Version::V1_17 => write!(f, "1.17"),
            Version::V1_17_1 => write!(f, "1.17.1"),
            Version::V1_18 => write!(f, "1.18"),
            Version::V1_18_1 => write!(f, "1.18.1"),
            Version::V1_18_2 => write!(f, "1.18.2"),
            Version::V1_19 => write!(f, "1.19"),
            Version::V1_19_1 => write!(f, "1.19.1"),
            Version::V1_19_2 => write!(f, "1.19.2"),
            Version::V1_20 => write!(f, "1.20"),
            Version::V1_20_1 => write!(f, "1.20.1"),
            Version::V1_20_2 => write!(f, "1.20.2"),
            Version::V1_20_3 => write!(f, "1.20.3"),
            Version::V1_20_4 => write!(f, "1.20.4"),
            Version::V1_20_5 => write!(f, "1.20.5"),
            Version::V1_21 => write!(f, "1.21"),
            Version::V1_21_1 => write!(f, "1.21.1"),
            Version::V1_21_2 => write!(f, "1.21.2"),
            Version::V1_21_3 => write!(f, "1.21.3"),
            Version::V1_21_4 => write!(f, "1.21.4"),
            Version::V1_21_5 => write!(f, "1.21.5"),
            Version::V1_21_6 => write!(f, "1.21.6"),
            Version::V1_21_7 => write!(f, "1.21.7"),
        }
    }   
}

pub enum AuthProtocol {
    Offline(String),
    // token, msa, profile
    Microsoft(String, Box<ExpiringValue<AccessTokenResponse>>, Box<MinecraftProfile>)
}

pub struct ControllerContainer {
    pub list: HashMap<Uuid, ClientController>,
}

impl Default for ControllerContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl ControllerContainer {
    pub fn new() -> Self {
        Self {
            list: HashMap::new(),
        }
    }
    
    pub fn add(&mut self, controller: ClientController) {
        info!("Registered controller for {}", controller.id);
        self.list.insert(controller.id, controller);
    }
    
    pub fn contains(&self, uuid: &Uuid) -> bool {
        self.list.contains_key(uuid)
    }
    
    pub fn remove(&mut self, uuid: &Uuid) {
        self.list.remove(uuid);
    }
    
    pub fn get(&self, uuid: &Uuid) -> Option<&ClientController> {
        self.list.get(uuid)
    }
    
    pub fn get_mut(&mut self, uuid: &Uuid) -> Option<&mut ClientController> {
        self.list.get_mut(uuid)
    }
}

pub struct ClientController {
    pub id: Uuid,
    pub username: String,
    pub uuid: Uuid,
    pub auth: Arc<AuthProtocol>,
    pub instances: HashMap<Uuid, ClientInstance>,
    pub logs_location: PathBuf
}

impl ClientController {
    pub fn new(id: Uuid, username: String, uuid: Uuid, auth: Arc<AuthProtocol>) -> Self {
        Self {
            id,
            username,
            uuid,
            auth,
            instances: HashMap::new(),
            logs_location: LOG_DIR.join(id.to_string())
        }
    }

    pub fn new_cached(api: &mut ApiContext, client_id: &Uuid, auth_cache: &MinecraftAuthCache)
        -> Result<Self, String> {
        let client = api.clients.get_by_id(client_id)
            .ok_or_else(|| format!("Could not find client {client_id} in local client register."))?;
        let profile = &auth_cache.profile;
        let mut controller = {
            ClientController::new(
                *client_id, profile.username.clone(), profile.uuid,
                Arc::new(AuthProtocol::Microsoft(
                    auth_cache.access_token.clone(),
                    Box::new(auth_cache.msa.clone()),
                    Box::new(profile.clone()),
                )),
            )
        };
        for (key, connection) in client.connections.iter() {
            controller.instances.insert(key.clone(), ClientInstance::new(
                key.clone(), profile.username.clone(), &profile.uuid, controller.auth.clone(),
                connection.server.clone(), Some(connection.version.clone()),
                controller.logs_location.clone()
            ));
        }

        Ok(controller)
    }

    pub fn create_instance(&mut self, server: Server, version: Option<Version>) -> Uuid {
        let id = Uuid::new_v4();
        let instance = {
            ClientInstance::new(id, self.username.clone(), &self.uuid,
                                self.auth.clone(), server, version,
                                self.logs_location.clone()
            )
        };
        self.instances.insert(id, instance);
        id
    }

    pub fn get_instance(&self, uuid: &Uuid) -> Option<&ClientInstance> {
        self.instances.get(uuid)
    }

    pub fn get_instance_mut(&mut self, uuid: &Uuid) -> Option<&mut ClientInstance> {
        self.instances.get_mut(uuid)
    }

    pub fn remove_instance(&mut self, uuid: &Uuid) {
        self.instances.remove(uuid);
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use crate::client::Version;

    #[test]
    fn version_string_conversion() {
        let ver = Version::from_str("1.20.5").unwrap();
        assert_eq!(ver, Version::V1_20_5);

        let str = Version::V1_16_5.to_string();
        assert_eq!(str.as_str(), "1.16.5");
    }
    
    #[test]
    fn versions_count() {
        let all = Version::all();
        assert_eq!(all.len(), 28);
        println!("{:?}", all);
    }
}
