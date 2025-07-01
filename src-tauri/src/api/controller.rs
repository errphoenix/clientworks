use std::{
    collections::HashMap,
    str::FromStr
};
use std::fmt::format;
use std::sync::Arc;
use azalea::Client;
use azalea::ecs::system::entity_command::insert;
use azalea::physics::clip::clip;
use tauri::State;
use uuid::Uuid;
use crate::{
    AppState,
    api::{
        ApiContext,
        client::ClientConnection
    },
    client::{
        Version,
        ClientInstance,
        hooks::Payload
    }
};
use crate::api::client::AuthType;
use crate::client::{AuthProtocol, ClientController};
use crate::client::auth::MinecraftProfile;
// Where present, the ID and KEY parameters represent the UUID of the client and controller, respectively.

#[tauri::command]
pub fn create_connection(
    ctx: State<'_, AppState>,
    id: String,
    server_name: String,
    version: String
) -> Result<String, String> {
    let uuid = Uuid::from_str(id.as_str()).unwrap();
    let version = Version::from_string(version.as_str());
    let instance_id: String = {
        let mut ctx = ctx.api_context.lock().unwrap();
        let mut server = ctx.servers.get_server(&server_name)
            .ok_or_else(|| format!("Server '{server_name}' not found"))?.clone();
        let mut controller = ctx.controllers.get_mut(&uuid)
            .ok_or_else(|| format!("Controller for client '{id}' not found"))?;
        let id = controller.create_instance(server.clone(), version.clone());
        {
            let conn = ClientConnection::new(
                id, version.unwrap_or_default(), server.clone()
            );
            let mut client = ctx.clients.get_mut_by_id(&uuid).unwrap();
            client.connections.insert(conn.id, conn);
        }
        id.to_string()
    };
    let ctx = ctx.api_context.lock().unwrap();
    ctx.clients.write_to_file(&ctx.save);
    Ok(instance_id)
}

#[tauri::command]
pub fn get_instances(
    ctx: State<'_, AppState>,
    id: String
) -> Result<HashMap<String, (bool, ClientConnection)>, String> {
    let mut ctx = ctx.api_context.lock().unwrap();
    let uuid = match Uuid::from_str(id.as_str()) {
        Ok(uuid) => uuid,
        Err(_) => return Err("Invalid UUID".to_string())
    };
    let client = {
        ctx.clients.get_by_id(&uuid).cloned()
    };
    if let Some(client) = client {
        let controller = {
            let id = Uuid::from_str(id.as_str()).unwrap();
            if let Some(controller) = ctx.controllers.get(&id) {
                controller
            } else {
                if client.auth == AuthType::Microsoft {
                    return Err("Controller not found".to_string())
                } else {
                    let profile = MinecraftProfile::with_username(client.username.clone());
                    let controller = ClientController::new(
                        client.id, client.username.clone(), profile.uuid,
                        Arc::new(AuthProtocol::Offline(client.username.clone()))
                    );
                    ctx.controllers.add(controller);
                    &ctx.controllers.get(&id).unwrap()
                }
            }
        };

        let map = {
            let mut map = HashMap::new();
            for (id, instance) in controller.instances.iter() {
                let connection = client.connections.get(id);
                if let Some(connection) = connection {
                    map.insert(id.to_string(), (instance.is_running(), connection.clone()));
                }
            }
            map
        };

        return Ok(map)
    }
    Err("Client not found".to_string())
}

#[tauri::command]
pub fn get_available_versions() -> Vec<Version> {
    Version::all()
}

fn locate_instance<'a>(
    api: &'a mut ApiContext,
    id: String, key: &Uuid
) -> Result<&'a mut ClientInstance, String> {
    let mut controller = {
        api.controllers.get_mut(&Uuid::from_str(id.as_str()).unwrap())
            .ok_or_else(|| format!("No client controller found from id: {id}"))?
    };
    controller.get_instance_mut(key)
        .ok_or_else(|| format!("No client instance found from key: {key}"))
}

#[tauri::command]
pub fn send_chat(
    ctx: State<'_, AppState>,
    id: String, key: String,
    message: String
) -> Result<(), String> {
    let key = Uuid::from_str(key.as_str())
        .map_err(|e| format!("{}", e.to_string()))?;
    let mut ctx = ctx.api_context.lock().unwrap();
    let mut instance = locate_instance(&mut ctx, id, &key)?;
    {
        if !instance.is_running() {
            return Err("Cannot send chat messages while the instance is offline [state]".to_owned());
        }
        instance.send_message(message);
    }
    Ok(())
}

#[tauri::command]
pub fn connect_client(
    ctx: State<'_, AppState>,
    id: String, key: String
) -> Result<(), String> {
    let key = Uuid::from_str(key.as_str())
        .map_err(|e| format!("{}", e.to_string()))?;
    {
        let mut ctx = ctx.api_context.lock().unwrap();
        let mut instance = locate_instance(&mut ctx, id, &key)?;
        instance.connect();
    }
    ctx.com_channel.lock().unwrap().send(
        key, Payload::Chat { message: "Received connect command...".to_string() }
    );
    Ok(())
}

#[tauri::command]
pub fn disconnect_client(
    ctx: State<'_, AppState>,
    id: String, key: String
) -> Result<(), String> {
    let key = Uuid::from_str(key.as_str())
        .map_err(|e| format!("{}", e.to_string()))?;
    ctx.com_channel.lock().unwrap().send(
        key, Payload::Chat { message: "Received disconnect command...".to_string() }
    );
    {
        let mut ctx = ctx.api_context.lock().unwrap();
        let mut instance = locate_instance(&mut ctx, id, &key)?;
        instance.disconnect_notify()?;
        // instance.disconnect()?;
    }
    Ok(())
}

#[tauri::command]
pub fn kill_client(
    ctx: State<'_, AppState>,
    id: String, key: String
) -> Result<(), String> {
    let key = Uuid::from_str(key.as_str())
        .map_err(|e| format!("{}", e.to_string()))?;
    ctx.com_channel.lock().unwrap().send(
        key, Payload::Chat { message: "Received force-kill command...".to_string() }
    );
    {
        let mut ctx = ctx.api_context.lock().unwrap();
        let mut instance = locate_instance(&mut ctx, id, &key)?;
        instance.kill()?;
    }
    Ok(())
}