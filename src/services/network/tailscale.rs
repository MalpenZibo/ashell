//! Tailscale backend for the network service
//!
//! Provides Tailscale VPN integration via the LocalAPI over Unix socket.

use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use hyperlocal::{UnixConnector, Uri};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

/// Default path to the Tailscale daemon socket
const TAILSCALE_SOCKET: &str = "/var/run/tailscale/tailscaled.sock";
/// Required Host header for LocalAPI security
const TAILSCALE_HOST: &str = "local-tailscaled.sock";

/// Current Tailscale state for the network service
#[derive(Debug, Clone, Default)]
pub struct TailscaleState {
    pub available: bool,
    pub is_running: bool,
    pub current_profile: Option<TailscaleProfile>,
    pub profiles: Vec<TailscaleProfile>,
    pub exit_nodes: Vec<ExitNode>,
    pub current_exit_node_id: Option<String>,
    pub allow_lan: bool,
}

/// A Tailscale profile (account/identity)
#[derive(Debug, Clone)]
pub struct TailscaleProfile {
    pub id: String,
    pub name: String,
    pub is_current: bool,
}

impl TailscaleProfile {
    /// Get display name (username only, without domain)
    pub fn display_name(&self) -> String {
        if let Some(at_pos) = self.name.find('@') {
            self.name[..at_pos].to_string()
        } else {
            self.name.clone()
        }
    }
}

/// Location information for an exit node
#[derive(Debug, Clone, Default)]
pub struct ExitNodeLocation {
    pub country: Option<String>,
    #[allow(dead_code)]
    pub country_code: Option<String>,
    pub city: Option<String>,
}

/// A peer that can be used as an exit node
#[derive(Debug, Clone)]
pub struct ExitNode {
    pub id: String,
    pub name: String,
    pub location: Option<ExitNodeLocation>,
    pub online: bool,
    #[allow(dead_code)]
    pub is_mullvad: bool,
}

impl ExitNode {
    /// Get country name for grouping
    pub fn country(&self) -> String {
        self.location
            .as_ref()
            .and_then(|l| l.country.clone())
            .unwrap_or_else(|| "Your Network".to_string())
    }

    /// Get display name (city + name or just name)
    pub fn display_name(&self) -> String {
        if let Some(loc) = &self.location {
            if let Some(city) = &loc.city {
                return format!("{} ({})", city, self.name);
            }
        }
        self.name.clone()
    }
}

// ============================================================================
// API Response Types (internal, for JSON deserialization)
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct ApiProfile {
    #[serde(rename = "ID")]
    id: String,
    name: String,
    #[serde(default)]
    current_profile: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct ApiStatus {
    #[serde(rename = "BackendState")]
    backend_state: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct ApiLocation {
    country: Option<String>,
    country_code: Option<String>,
    city: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiExitNode {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "DNSName")]
    dns_name: String,
    host_name: Option<String>,
    location: Option<ApiLocation>,
    online: bool,
    #[serde(default)]
    exit_node_option: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct ApiPrefs {
    #[serde(rename = "ExitNodeID")]
    exit_node_id: Option<String>,
    #[serde(rename = "ExitNodeAllowLANAccess", default)]
    exit_node_allow_lan_access: bool,
    #[serde(rename = "WantRunning", default)]
    want_running: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "PascalCase")]
struct ApiPrefsUpdate {
    #[serde(rename = "ExitNodeIDSet", skip_serializing_if = "Option::is_none")]
    exit_node_id_set: Option<bool>,
    #[serde(rename = "ExitNodeID", skip_serializing_if = "Option::is_none")]
    exit_node_id: Option<String>,
    #[serde(
        rename = "ExitNodeAllowLANAccessSet",
        skip_serializing_if = "Option::is_none"
    )]
    exit_node_allow_lan_access_set: Option<bool>,
    #[serde(
        rename = "ExitNodeAllowLANAccess",
        skip_serializing_if = "Option::is_none"
    )]
    exit_node_allow_lan_access: Option<bool>,
    #[serde(rename = "WantRunningSet", skip_serializing_if = "Option::is_none")]
    want_running_set: Option<bool>,
    #[serde(rename = "WantRunning", skip_serializing_if = "Option::is_none")]
    want_running: Option<bool>,
}

#[derive(Deserialize)]
struct StatusWithPeers {
    #[serde(rename = "Peer", default)]
    peer: HashMap<String, ApiExitNode>,
}

// ============================================================================
// Tailscale Backend
// ============================================================================

/// Backend for communicating with Tailscale LocalAPI
pub struct TailscaleBackend;

impl TailscaleBackend {
    /// Check if the Tailscale socket exists and is accessible
    pub fn is_available() -> bool {
        Path::new(TAILSCALE_SOCKET).exists()
    }

    /// Get the complete Tailscale state
    pub async fn get_state() -> Option<TailscaleState> {
        if !Self::is_available() {
            return None;
        }

        let status = match Self::get_status().await {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to get Tailscale status: {}", e);
                return None;
            }
        };

        let profiles = Self::get_profiles().await.unwrap_or_default();
        let exit_nodes = Self::get_exit_nodes().await.unwrap_or_default();
        let prefs = Self::get_prefs().await.unwrap_or_default();

        Some(TailscaleState {
            available: true,
            is_running: status.backend_state == "Running",
            current_profile: profiles.iter().find(|p| p.is_current).cloned(),
            profiles,
            exit_nodes,
            current_exit_node_id: prefs
                .exit_node_id
                .filter(|id| !id.is_empty()),
            allow_lan: prefs.exit_node_allow_lan_access,
        })
    }

    /// Connect to Tailscale (tailscale up)
    pub async fn connect() -> anyhow::Result<()> {
        let update = ApiPrefsUpdate {
            want_running_set: Some(true),
            want_running: Some(true),
            ..Default::default()
        };
        Self::set_prefs(&update).await?;
        debug!("Tailscale connected");
        Ok(())
    }

    /// Disconnect from Tailscale (tailscale down)
    pub async fn disconnect() -> anyhow::Result<()> {
        let update = ApiPrefsUpdate {
            want_running_set: Some(true),
            want_running: Some(false),
            ..Default::default()
        };
        Self::set_prefs(&update).await?;
        debug!("Tailscale disconnected");
        Ok(())
    }

    /// Switch to a different profile
    pub async fn switch_profile(profile_id: &str) -> anyhow::Result<()> {
        let endpoint = format!("/localapi/v0/profiles/{}", profile_id);
        Self::post(&endpoint).await?;
        debug!("Tailscale switched to profile: {}", profile_id);
        Ok(())
    }

    /// Set exit node (None to clear)
    pub async fn set_exit_node(node_id: Option<&str>) -> anyhow::Result<()> {
        let update = ApiPrefsUpdate {
            exit_node_id_set: Some(true),
            exit_node_id: Some(node_id.unwrap_or("").to_string()),
            ..Default::default()
        };
        Self::set_prefs(&update).await?;
        debug!("Tailscale exit node set to: {:?}", node_id);
        Ok(())
    }

    /// Set allow LAN access
    pub async fn set_allow_lan(allow: bool) -> anyhow::Result<()> {
        let update = ApiPrefsUpdate {
            exit_node_allow_lan_access_set: Some(true),
            exit_node_allow_lan_access: Some(allow),
            ..Default::default()
        };
        Self::set_prefs(&update).await?;
        debug!("Tailscale allow LAN set to: {}", allow);
        Ok(())
    }

    /// Watch for Tailscale state changes using the watch-ipn-bus streaming API.
    /// Returns a channel receiver that yields state updates.
    /// The stream will close if the daemon shuts down or connection is lost.
    pub async fn watch_state(
        sender: tokio::sync::mpsc::Sender<TailscaleState>,
    ) -> anyhow::Result<()> {
        use tokio::io::AsyncBufReadExt;
        use tokio::io::BufReader;
        
        if !Self::is_available() {
            anyhow::bail!("Tailscale socket not available");
        }

        // Connect to the Unix socket directly for streaming
        let socket = tokio::net::UnixStream::connect(TAILSCALE_SOCKET).await?;
        let (reader, mut writer) = tokio::io::split(socket);

        // Send HTTP request manually for the streaming endpoint
        let request = format!(
            "GET /localapi/v0/watch-ipn-bus HTTP/1.1\r\n\
             Host: {}\r\n\
             Connection: keep-alive\r\n\
             \r\n",
            TAILSCALE_HOST
        );
        
        use tokio::io::AsyncWriteExt;
        writer.write_all(request.as_bytes()).await?;

        let mut buf_reader = BufReader::new(reader);
        
        // Skip HTTP response headers
        let mut header_line = String::new();
        loop {
            header_line.clear();
            let bytes_read = buf_reader.read_line(&mut header_line).await?;
            if bytes_read == 0 || header_line.trim().is_empty() {
                break;
            }
        }

        debug!("Tailscale watch-ipn-bus connected");

        // Read newline-delimited JSON messages
        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = buf_reader.read_line(&mut line).await?;
            
            if bytes_read == 0 {
                // Connection closed
                debug!("Tailscale watch-ipn-bus connection closed");
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Each message indicates a state change, fetch new state
            // (The watch-ipn-bus messages are complex, easier to just refresh)
            if let Some(state) = Self::get_state().await {
                if sender.send(state).await.is_err() {
                    // Receiver dropped
                    break;
                }
            }
        }

        Ok(())
    }

    // ========================================================================
    // Internal API helpers
    // ========================================================================

    async fn get_status() -> anyhow::Result<ApiStatus> {
        let body = Self::get("/localapi/v0/status").await?;
        let status: ApiStatus = serde_json::from_str(&body)?;
        Ok(status)
    }

    async fn get_profiles() -> anyhow::Result<Vec<TailscaleProfile>> {
        let body = Self::get("/localapi/v0/profiles/").await?;
        let api_profiles: Vec<ApiProfile> = serde_json::from_str(&body)?;

        // Get current profile to mark it
        let current_id = Self::get_current_profile_id().await.ok();

        let profiles = api_profiles
            .into_iter()
            .map(|p| TailscaleProfile {
                is_current: current_id.as_ref() == Some(&p.id),
                id: p.id,
                name: p.name,
            })
            .collect();

        Ok(profiles)
    }

    async fn get_current_profile_id() -> anyhow::Result<String> {
        let body = Self::get("/localapi/v0/profiles/current").await?;
        let profile: ApiProfile = serde_json::from_str(&body)?;
        Ok(profile.id)
    }

    async fn get_exit_nodes() -> anyhow::Result<Vec<ExitNode>> {
        let body = Self::get("/localapi/v0/status").await?;
        let status: StatusWithPeers = serde_json::from_str(&body)?;

        let exit_nodes = status
            .peer
            .into_values()
            .filter(|p| p.exit_node_option)
            .map(|p| ExitNode {
                id: p.id,
                name: p.host_name.unwrap_or_else(|| p.dns_name.clone()),
                is_mullvad: p.dns_name.contains(".mullvad.ts.net"),
                online: p.online,
                location: p.location.map(|l| ExitNodeLocation {
                    country: l.country,
                    country_code: l.country_code,
                    city: l.city,
                }),
            })
            .collect();

        Ok(exit_nodes)
    }

    async fn get_prefs() -> anyhow::Result<ApiPrefs> {
        let body = Self::get("/localapi/v0/prefs").await?;
        let prefs: ApiPrefs = serde_json::from_str(&body)?;
        Ok(prefs)
    }

    async fn set_prefs(update: &ApiPrefsUpdate) -> anyhow::Result<ApiPrefs> {
        let body = Self::patch("/localapi/v0/prefs", update).await?;
        let prefs: ApiPrefs = serde_json::from_str(&body)?;
        Ok(prefs)
    }

    // ========================================================================
    // HTTP client helpers
    // ========================================================================

    fn create_client() -> Client<UnixConnector, Empty<Bytes>> {
        Client::builder(TokioExecutor::new()).build(UnixConnector)
    }

    async fn get(endpoint: &str) -> anyhow::Result<String> {
        let client = Self::create_client();
        let uri: hyper::Uri = Uri::new(TAILSCALE_SOCKET, endpoint).into();

        let request = hyper::Request::builder()
            .method("GET")
            .uri(uri)
            .header("Host", TAILSCALE_HOST)
            .body(Empty::<Bytes>::new())?;

        let response = client.request(request).await?;

        if !response.status().is_success() {
            anyhow::bail!("LocalAPI returned status: {}", response.status());
        }

        let body = response.into_body().collect().await?.to_bytes();
        Ok(String::from_utf8_lossy(&body).to_string())
    }

    async fn post(endpoint: &str) -> anyhow::Result<String> {
        let client = Self::create_client();
        let uri: hyper::Uri = Uri::new(TAILSCALE_SOCKET, endpoint).into();

        let request = hyper::Request::builder()
            .method("POST")
            .uri(uri)
            .header("Host", TAILSCALE_HOST)
            .body(Empty::<Bytes>::new())?;

        let response = client.request(request).await?;

        if !response.status().is_success() {
            anyhow::bail!("LocalAPI returned status: {}", response.status());
        }

        let body = response.into_body().collect().await?.to_bytes();
        Ok(String::from_utf8_lossy(&body).to_string())
    }

    async fn patch<T: Serialize>(endpoint: &str, body: &T) -> anyhow::Result<String> {
        use http_body_util::Full;
        use hyper_util::client::legacy::Client as LegacyClient;

        let json_body = serde_json::to_string(body)?;
        let uri: hyper::Uri = Uri::new(TAILSCALE_SOCKET, endpoint).into();

        let client: LegacyClient<UnixConnector, Full<Bytes>> =
            LegacyClient::builder(TokioExecutor::new()).build(UnixConnector);

        let request = hyper::Request::builder()
            .method("PATCH")
            .uri(uri)
            .header("Host", TAILSCALE_HOST)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(json_body)))?;

        let response = client.request(request).await?;

        if !response.status().is_success() {
            anyhow::bail!("LocalAPI PATCH returned status: {}", response.status());
        }

        let body = response.into_body().collect().await?.to_bytes();
        Ok(String::from_utf8_lossy(&body).to_string())
    }
}
