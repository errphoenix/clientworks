use crate::{
    api::ApiContext,
    client::{
        AuthProtocol,
        ClientController,
        auth::{
            MinecraftProfile,
            self, AuthState
        }
    },
    AppState
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, path::Path,
    sync::Mutex,
    time::{
        SystemTime,
        UNIX_EPOCH
    },
    sync::Arc,
    str::FromStr,
    ops::DerefMut
};
use azalea::{
    Account,
    ecs::error::info
};
use azalea_auth::{
    AccessTokenResponse,
    cache::ExpiringValue
};
use log::info;
use tauri::{
    AppHandle,
    Emitter,
    State
};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Clone)]
pub struct MinecraftAuthCache {
    pub access_token: String,
    pub expiration: u64,
    pub msa: ExpiringValue<AccessTokenResponse>,
    pub profile: MinecraftProfile
}

impl MinecraftAuthCache {
    pub fn has_expired(&self) -> bool {
        self.expiration
            < SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct AuthCache(HashMap<String, MinecraftAuthCache>);

impl AuthCache {
    pub fn from_file(path: &Path) -> Self {
        let path = path.join("auth_cache.json");
        if !path.exists() {
            fs::write(&path, "{}");
        }
        let file = fs::read_to_string(path).unwrap_or_default();
        let auth_cache: AuthCache = serde_json::from_str(&file).unwrap_or_default();
        info!("Cached accounts: {} [{:?}]", auth_cache.0.len(), auth_cache.0.keys());
        auth_cache
    }

    pub fn write_to_file(&self, path: &Path) {
        let path = path.join("auth_cache.json");
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(&path, json).unwrap();
    }

    pub fn get_from_mc_uuid(&self, uuid: &Uuid) -> Option<&MinecraftAuthCache> {
        for (key, cache) in self.0.iter() {
            if cache.profile.uuid.eq(uuid) {
                return Some(cache)
            }
        }
        None
    }

    pub fn get_key_from_mc_uuid(&self, uuid: &Uuid) -> Option<&String> {
        for (key, cache) in self.0.iter() {
            if cache.profile.uuid.eq(uuid) {
                return Some(key)
            }
        }
        None
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthProgress {
    state: String,
    message: String,
}

impl From<&AuthState> for AuthProgress {
    fn from(value: &AuthState) -> Self {
        let (name, message) = match value {
            AuthState::Working(msg) => ("Working", msg.to_string()),
            AuthState::Success(msg) => ("Success", msg.to_string()),
            AuthState::Error(msg) => ("Error", msg.to_string()),
        };
        Self {
            state: name.to_string(),
            message,
        }
    }
}

fn emit_progress_event(app: &AppHandle, state: &AuthState) {
    let progress = AuthProgress::from(state);
    app.emit("auth-progress-update", progress);
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthCredentials {
    uri: String,
    code: String,
}

#[tauri::command]
pub fn auth_validity(
    app: AppHandle,
    ctx: State<'_, AppState>,
    uuid: String
) -> u64 {
    let mut ctx = ctx.api_context.lock().unwrap();
    let uuid = Uuid::from_str(uuid.as_str()).unwrap();
    if let Some(cache) = ctx.auth_cache.get_from_mc_uuid(&uuid) {
        return cache.expiration
    }
    0
}

#[tauri::command]
pub async fn recall_authentication(
    app: AppHandle,
    ctx: State<'_, AppState>,
    id: String
) -> Result<bool, String> {
    let mut ctx = ctx.api_context.lock().unwrap();
    let uuid = {
        Uuid::from_str(id.as_str()).map_err(|err| { err.to_string() })?
    };

    if ctx.controllers.get(&uuid).is_some() {
        Ok(true)
    } else {
        // TODO add an hyperlink to the 'report a bug' text
        let label_bug_report = "<u className=\"text-red-500\">Report a bug</u> if you believe this is an error.";

        let client = ctx.clients.get_by_id(&uuid);
        if let Some(client) = client {
            let key = {
                ctx.auth_cache.get_key_from_mc_uuid(&client.uuid)
                    .ok_or_else(|| format!(
                        r#"<div>No authentication key found in cache for client with ID <u className=\"text-red-400\">{}</u>.
                        <br />Please check your account cache in <u className=\"text-red-400\">auth_cache.json</u> if allowed to.
                        <br /> <br />
                        {label_bug_report}</div>"#,
                        client.uuid
                    ))?.clone()
            };
            let result = cached_authentication(app, &mut ctx, &key);
            match result {
                Ok(_) => Ok(true),
                Err(e) => Err(format!("<div>{e}<br /><br />{label_bug_report}</div>"))
            }
        } else {
            Err(format!("<div>No client registered with ID: {uuid}<br /><br />{label_bug_report}</div>"))
        }
    }
}

#[tauri::command]
pub async fn auth_offline(
    app: AppHandle,
    ctx: State<'_, AppState>,
    username: String
) -> Result<(String, MinecraftProfile), String> {
    let mut ctx = ctx.api_context.lock().unwrap();
    emit_progress_event(&app, &AuthState::Working("Verifying account...".to_string()));
    if ctx.clients.get_by_username(&username).is_some() {
        emit_progress_event(&app, &AuthState::Error(format!("Account {username} is already registered.")));
        return Err("Account already exists.".to_string())
    }
    emit_progress_event(&app, &AuthState::Working("Offline account created.".to_string()));
    let profile = MinecraftProfile::with_username(username);
    let id = crate::api::client::register(&mut ctx, &profile)?.to_string();
    Ok((id, profile))
}

fn cached_authentication(
    app: AppHandle,
    api_context: &mut ApiContext,
    login_key: &String,
) -> Result<(String, MinecraftProfile), String> {
    emit_progress_event(&app, &AuthState::Working("Looking for cache...".to_string()));
    let cache = {
        let cache = api_context.auth_cache.0.get(login_key);
        if let Some(cache) = cache {
            if cache.has_expired() {
                emit_progress_event(&app, &AuthState::Error("Cache expired, re-authentication is required.".to_string()));
                None
            } else {
                emit_progress_event(&app, &AuthState::Working("Valid cache found.".to_string()));
                Some(cache.clone())
            }
        } else {
            emit_progress_event(&app, &AuthState::Error("No cache found.".to_string()));
            None
        }
    };
    emit_progress_event(&app, &AuthState::Working("Validating cache...".to_string()));
    if let Some(cache) = cache {
        let client_id = {
            let uuid = &cache.profile.uuid;
            if let Some(client) = api_context.clients.get_by_mc_uuid(uuid) {
                &client.id.clone()
            } else {
                emit_progress_event(&app, &AuthState::Working("Registering new client from cached profile...".to_string()));
                &crate::api::client::register(api_context, &cache.profile)?
            }
        };
        emit_progress_event(&app, &AuthState::Success("Cache successfully validated, authentication is allowed.".to_string()));
        let controller = ClientController::new_cached(api_context, client_id, &cache)?;
        api_context.controllers.add(controller);
        let profile = &cache.profile;
        return Ok((client_id.to_string(), profile.clone()));
    }
    emit_progress_event(&app, &AuthState::Error("Account not found in cache.".to_string()));
    Err("Account not found in cache or cached token(s) have expired.".to_string())
}

#[tauri::command]
pub async fn auth_ms_cache(
    app: AppHandle,
    ctx: State<'_, AppState>,
    login_key: String,
) -> Result<(String, MinecraftProfile), String> {
    let mut guard = ctx.api_context.lock().unwrap();
    cached_authentication(app, &mut guard, &login_key)
}

#[tauri::command]
pub async fn auth_ms_init(
    app: AppHandle,
    ctx: State<'_, AppState>,
    login_key: String,
) -> Result<AuthCredentials, String> {
    let mut auth = auth::Authentication::new();
    auth.get_access_info(|state| {
        emit_progress_event(&app, state);
    })
    .await;

    if let Some(credentials) = &auth.credentials {
        let (uri, code) = (credentials.uri.clone(), credentials.code.clone());
        println!("{credentials:#?}");
        ctx.api_context.lock().unwrap().ongoing_auths.insert(login_key, auth);
        Ok(AuthCredentials { uri, code })
    } else {
        Err(auth.state.to_string())
    }
}

#[tauri::command]
pub async fn auth_ms_finish(
    app: AppHandle,
    ctx: State<'_, AppState>,
    login_key: String,
    register: bool
) -> Result<(String, MinecraftProfile), String> {
    let mut auth = {
        let mut ctx_guard = ctx.api_context.lock().unwrap();
        ctx_guard.ongoing_auths.remove(&login_key)
    };

    if let Some(mut auth) = auth {
        auth.authenticate_ms(Default::default(), |state| {
            emit_progress_event(&app, state);
        })
        .await;
        auth.authenticate_minecraft(|state| {
            emit_progress_event(&app, state);
        })
        .await;

        if let Some(token) = &auth.access_token {
            if let Some(profile) = auth.profile {
                let id = {
                    let mut ctx = ctx.api_context.lock().unwrap();
                    if register {
                        let msa = auth.msa.unwrap();
                        let cache = MinecraftAuthCache {
                            access_token: token.mca.data.access_token.clone(),
                            msa: msa.clone(),
                            expiration: token.mca.expires_at,
                            profile: profile.clone()
                        };
                        ctx.auth_cache.0.insert(login_key.clone(), cache);
                        ctx.auth_cache.write_to_file(&ctx.save);
                        let id = crate::api::client::register(&mut ctx, &profile)?;
                        let controller = ClientController::new(
                            id, profile.username.clone(), profile.uuid,
                            Arc::new(AuthProtocol::Microsoft(
                                token.mca.data.access_token.clone(),
                                Box::new(msa), Box::new(profile.clone())
                            )),
                        );
                        ctx.controllers.add(controller);
                        id.to_string()
                    } else {
                        "".to_string()
                    }
                };
                Ok((id, profile))
            } else {
                Err("No profile found from account.".to_string())
            }
        } else {
            Err(auth.state.to_string())
        }
    } else {
        Err(format!("No ongoing auth found from provided login key: {login_key}"))
    }
}
