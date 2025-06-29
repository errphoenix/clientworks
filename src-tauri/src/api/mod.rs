#![allow(unused)]

use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
    str::FromStr,
    ops::DerefMut
};
use tauri::State;
use tokio::fs;
use uuid::Uuid;

pub mod auth;
mod client;
pub mod controller;
mod server;

pub use server::{
    List as ServerList, Server,
};
pub use client::{
    List as ClientList, Client,
};

use crate::{
    api::{
        auth::AuthCache,
        client::AuthType::Microsoft
    },
    client::{
        ClientController,
        ControllerContainer,
        auth::Authentication
    },
    AppState
};
use crate::api::client::ClientConnection;
use crate::client::Version;

pub struct ApiContext {
    pub controllers: ControllerContainer,
    pub clients: ClientList,
    pub servers: ServerList,
    pub save: PathBuf,
    pub ongoing_auths: HashMap<String, Authentication>,
    pub auth_cache: AuthCache
}

pub fn load_from_dir(path: PathBuf) -> ApiContext {
    info!("Initialised API context from directory: {path:?}");
    ApiContext {
        controllers: ControllerContainer::new(),
        clients: ClientList::from_file(&path),
        servers: ServerList::from_file(&path),
        auth_cache: AuthCache::from_file(&path),
        save: path,
        ongoing_auths: HashMap::new()
    }
}

#[derive(Serialize, Debug)]
pub struct ClientInfo {
    id: String,
    username: String,
    auth: bool,
    uuid: String,
    instance_count: usize
}

#[tauri::command]
pub fn remove_client(ctx: State<'_, AppState>, uuid: String) -> Result<(), String> {
    let mut ctx = ctx.api_context.lock().unwrap();
    client::unregister(&mut ctx, uuid)
}

fn map_client_info(client: &mut Client) -> ClientInfo {
    ClientInfo {
        id: client.id.to_string(),
        username: client.username.clone(),
        auth: client.auth == Microsoft,
        uuid: client.uuid.to_string(),
        instance_count: client.connections.len()
    }
}

#[tauri::command]
pub fn get_client(ctx: State<'_, AppState>, id: String) -> Option<ClientInfo> {
    let ctx = ctx.api_context.lock().unwrap();
    ctx.clients
        .0
        .values()
        .find(|client| client.id.to_string() == id)
        .cloned()
        .map(|mut client| map_client_info(&mut client))
}

#[tauri::command]
pub fn get_client_by_user(
    ctx: State<'_, AppState>,
    username: String,
) -> Option<ClientInfo> {
    todo!()
}

#[tauri::command]
pub fn get_clients(ctx: State<'_, AppState>) -> Vec<ClientInfo> {
    let ctx = ctx.api_context.lock().unwrap();
    ctx.clients
        .0
        .values()
        .cloned()
        .map(|mut client| map_client_info(&mut client))
        .collect()
}

#[derive(Serialize, Debug)]
pub struct ServerInfo {
    name: String,
    ip: String,
    port: u16,
    connections: u32,
}

#[tauri::command]
pub fn add_server(
    ctx: State<'_, AppState>,
    name: String,
    ip: String,
    port: u16,
) -> Result<(), String> {
    let mut ctx = ctx.api_context.lock().unwrap();
    server::create(&mut ctx, name, ip, port)
}

#[tauri::command]
pub fn delete_server(ctx: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut ctx = ctx.api_context.lock().unwrap();
    server::delete(&mut ctx, name)
}

#[tauri::command]
pub fn get_servers(ctx: State<'_, AppState>) -> Vec<ServerInfo> {
    let ctx = ctx.api_context.lock().unwrap();
    ctx.servers
        .0
        .values()
        .cloned()
        .map(|server| ServerInfo {
            name: server.name.clone(),
            ip: server.ip.clone(),
            port: server.port,
            connections: 0,
        })
        .collect()
}
