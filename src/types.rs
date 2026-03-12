use serde::Deserialize;
use std::time::Duration;

/// Complete result of a server status probe.
#[derive(Debug, Clone)]
pub struct ServerInfo {
	/// Server address (hostname or IP). Included for context in logging/future features.
	#[allow(dead_code)]
	pub address: String,
	/// Server port. Included for context in logging/future features.
	#[allow(dead_code)]
	pub port: u16,
	pub status: ServerStatus,
	pub latency: Duration,
	pub blocked: Option<bool>,
	/// Whether address was resolved via SRV lookup. Included for context in logging/future features.
	#[allow(dead_code)]
	pub srv: bool,
}

// ── SLP JSON Structures ──────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ServerStatus {
	pub version: Version,
	pub players: Players,
	pub description: Description,
	#[serde(default)]
	pub favicon: Option<String>,
	#[serde(default, rename = "enforcesSecureChat")]
	pub enforces_secure_chat: Option<bool>,
	#[serde(default, rename = "previewsChat")]
	pub previews_chat: Option<bool>,
	/// Forge/NeoForge embed mod data here.
	#[serde(default, rename = "forgeData")]
	pub forge_data: Option<serde_json::Value>,
	#[serde(default, rename = "modinfo")]
	pub mod_info: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Version {
	pub name: String,
	pub protocol: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Players {
	pub max: i32,
	pub online: i32,
	#[serde(default)]
	pub sample: Vec<PlayerSample>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerSample {
	pub name: String,
	pub id: String,
}

// ── Chat Component (MOTD) ────────────────────────────

/// The `description` field can be a plain string or a chat component tree.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Description {
	Plain(String),
	Component(ChatComponent),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatComponent {
	#[serde(default)]
	pub text: String,
	#[serde(default)]
	pub translate: Option<String>,
	#[serde(default)]
	pub extra: Vec<ChatComponent>,
	#[serde(default)]
	pub color: Option<String>,
	#[serde(default)]
	pub bold: Option<bool>,
	#[serde(default)]
	pub italic: Option<bool>,
	#[serde(default)]
	pub underlined: Option<bool>,
	#[serde(default)]
	pub strikethrough: Option<bool>,
	#[serde(default)]
	pub obfuscated: Option<bool>,
}
