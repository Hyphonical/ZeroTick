use crate::config::MOJANG_SESSION_API;
use crate::error::{Error, Result};
use crate::net;
use base64::Engine;
use serde::Deserialize;

/// A resolved player profile with optional skin/cape URLs.
#[derive(Debug, Clone)]
pub struct PlayerProfile {
	/// Player's username. Not yet used; reserved for display in future UI features.
	#[allow(dead_code)]
	pub name: String,
	/// Player's UUID (de-hyphenated). Not yet used; reserved for future features.
	#[allow(dead_code)]
	pub uuid: String,
	pub skin_url: Option<String>,
	pub cape_url: Option<String>,
}

#[derive(Deserialize)]
struct SessionProfile {
	name: String,
	id: String,
	#[serde(default)]
	properties: Vec<ProfileProperty>,
}

#[derive(Deserialize)]
struct ProfileProperty {
	name: String,
	value: String,
}

#[derive(Deserialize)]
struct TexturesPayload {
	textures: Textures,
}

#[derive(Deserialize)]
struct Textures {
	#[serde(default, rename = "SKIN")]
	skin: Option<TextureEntry>,
	#[serde(default, rename = "CAPE")]
	cape: Option<TextureEntry>,
}

#[derive(Deserialize)]
struct TextureEntry {
	url: String,
}

/// Fetch a player's profile (including skin/cape URLs) from Mojang's session server.
pub async fn fetch(uuid: &str) -> Result<PlayerProfile> {
	let clean_uuid: String = uuid.chars().filter(|c| *c != '-').collect();

	let url = format!("{MOJANG_SESSION_API}/session/minecraft/profile/{clean_uuid}");
	let body = net::https::get(&url).await?;

	let profile: SessionProfile =
		serde_json::from_str(&body).map_err(|e| Error::MojangApi(e.to_string()))?;

	let mut skin_url = None;
	let mut cape_url = None;

	for prop in &profile.properties {
		if prop.name == "textures" {
			if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&prop.value) {
				if let Ok(payload) = serde_json::from_slice::<TexturesPayload>(&decoded) {
					skin_url = payload.textures.skin.map(|t| t.url);
					cape_url = payload.textures.cape.map(|t| t.url);
				}
			}
		}
	}

	Ok(PlayerProfile {
		name: profile.name,
		uuid: profile.id,
		skin_url,
		cape_url,
	})
}
