//! Types and paths shared by the native host and the launcher plugin.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A browser tab, as reported by the extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: i64,
    pub window_id: i64,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub active: bool,
    /// Content hash of the tab's favicon, previously sent in an `icon` message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_id: Option<String>,
}

/// Request sent by the plugin to a host over its socket, as one line of JSON.
/// The host replies to `List` with a JSON array of [`Tab`]s and to `Activate`
/// with nothing, then closes the connection.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SocketRequest {
    List,
    #[serde(rename_all = "camelCase")]
    Activate { tab_id: i64, window_id: i64 },
}

/// Directory holding one Unix socket per running native host (one per browser profile).
pub fn socket_dir() -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    runtime.join("cosmic-browser-tabs")
}

/// Directory holding favicons as `<sha256 hex>.png`, shared by all hosts.
pub fn icon_dir() -> PathBuf {
    socket_dir().join("icons")
}

/// Icon ids come from the browser and end up in file names: only accept a
/// lowercase SHA-256 hex digest so they can't escape [`icon_dir`].
pub fn is_valid_icon_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

pub fn icon_path(id: &str) -> PathBuf {
    icon_dir().join(format!("{id}.png"))
}
