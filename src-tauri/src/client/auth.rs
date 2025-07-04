use azalea_auth::{AccessTokenResponse, DeviceCodeResponse, MinecraftTokenResponse, cache::ExpiringValue, ProfileResponse, RefreshMicrosoftAuthTokenError};
use std::{
    fmt::Display,
    time::Duration
};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct VerificationInfo {
    pub code: String,
    pub uri: String,
    pub device: String,
    pub expiration: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfile {
    pub uuid: Uuid,
    pub username: String,
    pub skins: Option<Vec<serde_json::Value>>,
    pub capes: Option<Vec<serde_json::Value>>,
    pub authenticated: bool
}

impl From<&ProfileResponse> for MinecraftProfile {
    fn from(value: &ProfileResponse) -> Self {
        Self {
            uuid: value.id,
            username: value.name.clone(),
            skins: Some(value.skins.clone()),
            capes: Some(value.capes.clone()),
            authenticated: true
        }
    }
}

impl MinecraftProfile {
    pub fn with_username(username: String) -> Self {
        // generate uuid from `OfflinePlayer:<username>`
        let uuid = Uuid::new_v3(
            &Uuid::NAMESPACE_X500,
            format!("OfflinePlayer:{username}")
                .as_bytes()
        );
        info!("Generated offline UUID for {username}: {uuid}");

        Self {
            uuid,
            username,
            skins: None,
            capes: None,
            authenticated: false
        }
    }
}

/// Attempts to refresh the provided MSA token, using its refresh token.
/// This method does not verify whether the MSA token has expired or not.
///
/// # Parameters
/// * `state_callback` - a callback passing a reference to [`AuthState`] as an argument which
///   can be useful to display the current state of the authentication process to the user.
///   Provide an empty callback `|_| {}` if you don't want to display anything.
/// * `msa` - the MSA token to refresh
///
/// # Returns
/// A result containing a valid MSA token or a [`azalea_auth::RefreshMicrosoftAuthTokenError`] error
pub async fn refresh_ms<Scb>(
    mut state_callback: Scb,
    msa: &ExpiringValue<AccessTokenResponse>,
) -> Result<ExpiringValue<AccessTokenResponse>, RefreshMicrosoftAuthTokenError>
where
    Scb: FnMut(&AuthState),
{
    if cfg!(debug_assertions) { debug!("Requested token refresh...") }
    match azalea_auth::refresh_ms_auth_token(
        &reqwest::Client::new(),
        &msa.data.refresh_token,
        None, None
    ).await {
        Ok(msa) => {
            state_callback(&AuthState::Working("Successfully refreshed MSA token".to_owned()));
            Ok(msa)
        },
        Err(e) => {
            state_callback(&AuthState::Error(format!(
                "Failed to refresh MSA token. Re-authentication is required. ({e})"
            )));
            Err(e)
        }
    }
}

pub struct Authentication {
    client: reqwest::Client,
    pub credentials: Option<VerificationInfo>,
    pub msa: Option<ExpiringValue<AccessTokenResponse>>,
    pub access_token: Option<MinecraftTokenResponse>,
    pub profile: Option<MinecraftProfile>,
    pub state: AuthState,
}

impl From<&VerificationInfo> for DeviceCodeResponse {
    fn from(value: &VerificationInfo) -> Self {
        Self {
            user_code: value.code.clone(),
            verification_uri: value.uri.clone(),
            device_code: value.device.clone(),
            expires_in: value.expiration,
            interval: value.interval,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AuthState {
    /// The authenticator is currently working on something.
    /// Contains a user-friendly message about what's currently going on.
    Working(String),
    /// The authentication process has completed successfully.
    /// Contains the access token of the authenticated Minecraft session.
    Success(String),
    /// The authentication process has failed.
    /// Contains a user-friendly error message as a String
    Error(String),
}

impl Display for AuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            AuthState::Working(msg) => msg.clone(),
            AuthState::Success(token) => format!("Got Minecraft session token: [{token}]"),
            AuthState::Error(msg) => msg.clone(),
        };
        write!(f, "{str}")
    }
}

pub struct AuthTimeout(u64);

impl AuthTimeout {
    pub fn new(timeout: u64) -> Self {
        Self(timeout)
    }

    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.0)
    }
}

impl From<Duration> for AuthTimeout {
    fn from(duration: Duration) -> Self {
        Self::new(duration.as_millis() as u64)
    }
}

impl Default for AuthTimeout {
    fn default() -> Self {
        Duration::from_secs(90).into()
    }
}

impl Default for Authentication {
    fn default() -> Self {
        Self::new()
    }
}

impl Authentication {
    /// Creates a new asynchronous authentication client
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            credentials: None,
            msa: None,
            profile: None,
            access_token: None,
            state: AuthState::Working("Authentication started, waiting for requests".to_string()),
        }
    }

    #[allow(dead_code)] // used in tests
    pub fn state_is_final(&self) -> bool {
        matches!(self.state, AuthState::Success(_) | AuthState::Error(_))
    }

    /// Gain access info (Microsoft verification URI and user verification access code)
    /// for the user to log in into their Microsoft account and allow the authenticator to
    /// continue with the login process.
    ///
    /// # Parameters
    /// * `state_callback` - a callback passing a reference to [`AuthState`] as an argument which
    ///   can be useful to display the current state of the authentication process to the user.
    ///   Provide an empty callback `|_| {}` if you don't want to display anything.
    ///
    /// # Returns
    /// The last [`AuthState`] the authenticator was left on, either an [`AuthState::Working`]
    /// containing the access info or [`AuthState::Error`] containing a
    /// [`azalea_auth::GetMicrosoftAuthTokenError`] as a String.
    pub async fn get_access_info<Scb>(&mut self, mut state_callback: Scb) -> &AuthState
    where
        Scb: FnMut(&AuthState),
    {
        self.state = AuthState::Working("Getting access info...".to_string());
        state_callback(&self.state);
        match azalea_auth::get_ms_link_code(&self.client, None, None).await {
            Ok(code_resp) => {
                self.credentials = Some(VerificationInfo {
                    code: code_resp.user_code,
                    uri: code_resp.verification_uri,
                    device: code_resp.device_code,
                    expiration: code_resp.expires_in,
                    interval: code_resp.interval,
                });
                self.state = AuthState::Working("Got MS access credentials.".to_string());
                state_callback(&self.state);
            }
            Err(err) => {
                self.state = AuthState::Error(err.to_string());
                state_callback(&self.state);
            }
        }
        &self.state
    }

    /// Proceeds with the authentication process, authenticating the Microsoft account and
    /// gaining an access token.
    ///
    /// You must specify a timeout [`AuthTimeout`] after which this authentication step is
    /// aborted if *the user does not complete the previous step* within the specified timeout.
    ///
    /// # Parameters
    /// * `timeout` - The timeout for the *user verification process* after which this
    ///   authentication step is aborted, default is 90s; see [`AuthTimeout`]
    /// * `state_callback` - a callback passing a reference to [`AuthState`] as an argument which
    ///   can be useful to display the current state of the authentication process to the user.
    ///   Provide an empty callback `|_| {}` if you don't want to display anything.
    ///
    /// # Returns
    /// The last [`AuthState`] the authenticator was left on, either an [`AuthState::Working`]
    /// containing the access info or [`AuthState::Error`] containing a
    /// [`azalea_auth::GetMicrosoftAuthTokenError`] as a String.
    pub async fn authenticate_ms<Scb>(
        &mut self,
        timeout: AuthTimeout,
        mut state_callback: Scb,
    ) -> &AuthState
    where
        Scb: FnMut(&AuthState),
    {
        self.state = AuthState::Working("Waiting for User authentication...".to_string());
        state_callback(&self.state);
        println!("{:#?}", self.credentials);
        if let Some(resp) = &self.credentials {
            self.state = AuthState::Working("Authenticating Microsoft account...".to_string());
            state_callback(&self.state);
            let mut device_code: DeviceCodeResponse = resp.into();
            device_code.expires_in = timeout.duration().as_secs();
            match azalea_auth::get_ms_auth_token(&self.client, device_code, None).await {
                Ok(msa) => {
                    self.msa = Some(msa);
                    self.state = AuthState::Working(
                        "Got Microsoft access token, successfully authenticated!".to_string(),
                    );
                    state_callback(&self.state);
                }
                Err(err) => {
                    self.state = AuthState::Error(err.to_string());
                    state_callback(&self.state);
                }
            }
        } else {
            self.state = AuthState::Error("No access info to authenticate with".to_string());
            state_callback(&self.state);
        }
        &self.state
    }

    /// The last step in the authentication process, authenticating the Minecraft session and
    /// getting a session token.
    ///
    /// # Parameters
    /// * `state_callback` - a callback passing a reference to [`AuthState`] as an argument which
    ///   can be useful to display the current state of the authentication process to the user.
    ///   Provide an empty callback `|_| {}` if you don't want to display anything.
    ///
    /// # Returns
    /// The last [`AuthState`] the authenticator was left on, either an [`AuthState::Success`]
    /// containing the Minecraft session token or [`AuthState::Error`] containing a
    /// [`azalea_auth::AuthError`] as a String.
    pub async fn authenticate_minecraft<Scb>(&mut self, mut state_callback: Scb) -> &AuthState
    where
        Scb: FnMut(&AuthState),
    {
        self.state = AuthState::Working("Waiting for Microsoft authentication...".to_string());
        state_callback(&self.state);
        if let Some(msa) = &self.msa {
            self.state = AuthState::Working("Authenticating Minecraft session...".to_string());
            state_callback(&self.state);
            match azalea_auth::get_minecraft_token(&self.client, &msa.data.access_token).await {
                Ok(token) => {
                    self.state = AuthState::Working("Got session token, retrieving profile...".to_string());
                    match azalea_auth::get_profile(&self.client, &token.minecraft_access_token).await {
                        Ok(profile) => {
                            self.state = AuthState::Working(format!("Got profile: {}", profile.id));
                            self.profile = Some(MinecraftProfile::from(&profile));
                        }
                        Err(err) => {
                            self.state = AuthState::Error(err.to_string());
                        }
                    }
                    self.state = AuthState::Success(token.minecraft_access_token.clone());
                    self.access_token = Some(token);
                    state_callback(&self.state);
                }
                Err(err) => {
                    self.state = AuthState::Error(err.to_string());
                    state_callback(&self.state);
                }
            }
        } else {
            self.state = AuthState::Error("No MSA credentials to authenticate with".to_string());
            state_callback(&self.state);
        }
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use crate::client::auth::{AuthState, AuthTimeout, Authentication};
    use std::time::Duration;

    #[tokio::test]
    async fn test_full_process() {
        let mut auth = Authentication::new();
        auth.get_access_info(|_| {}).await;
        println!("Credentials: {:?}", auth.credentials);
        auth.authenticate_ms(Default::default(), |_| {}).await;
        println!("MSA: {:?}", auth.msa);
        auth.authenticate_minecraft(|_| {}).await;
        println!("Result: {:?}", auth.access_token);
        assert!(auth.state_is_final());
        assert_eq!(
            auth.state,
            AuthState::Success(format!(
                "Got Minecraft session token: [{}]",
                auth.access_token.unwrap().minecraft_access_token.clone()
            ))
        );
    }

    #[tokio::test]
    async fn test_timeout() {
        let mut auth = Authentication::new();
        auth.get_access_info(|_| {}).await;
        println!("Credentials: {:?}", auth.credentials);
        auth.authenticate_ms(AuthTimeout::from(Duration::from_secs(1)), |_| {})
            .await;
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(auth.state_is_final());
        assert_eq!(
            auth.state,
            AuthState::Error("Authentication timed out".to_string())
        );
    }

    #[tokio::test]
    async fn test_state_callback() {
        let mut auth = Authentication::new();
        auth.get_access_info(|state| {
            println!("Creds: {:?}", state);
        })
        .await;
        auth.authenticate_ms(Default::default(), |state| {
            println!("MS auth: {:?}", state);
        })
        .await;
        auth.authenticate_minecraft(|state| {
            println!("MC auth: {:?}", state);
        })
        .await;
    }
}
