use crate::config::MOJANG_BLOCKED_SERVERS;
use crate::error::Result;
use crate::net;
use sha1::{Digest, Sha1};
use std::collections::HashSet;

/// Check if a server address is on Mojang's blocked servers list.
pub async fn is_blocked(host: &str, port: u16) -> Result<bool> {
	let body = net::https::get(MOJANG_BLOCKED_SERVERS).await?;

	let hashes: HashSet<&str> = body
		.lines()
		.map(str::trim)
		.filter(|l| !l.is_empty())
		.collect();

	// Mojang checks several address variants
	let candidates = vec![
		host.to_lowercase(),
		format!("{}:{}", host.to_lowercase(), port),
		// Wildcard patterns: *.example.com
		{
			let parts: Vec<&str> = host.split('.').collect();
			if parts.len() > 2 {
				format!("*.{}", parts[1..].join(".").to_lowercase())
			} else {
				String::new()
			}
		},
	];

	for candidate in &candidates {
		if candidate.is_empty() {
			continue;
		}
		let hash = sha1_hex(candidate);
		if hashes.contains(hash.as_str()) {
			tracing::debug!("blocked match: {candidate} -> {hash}");
			return Ok(true);
		}
	}

	Ok(false)
}

fn sha1_hex(input: &str) -> String {
	let mut hasher = Sha1::new();
	hasher.update(input.as_bytes());
	let result = hasher.finalize();
	hex_encode(&result)
}

fn hex_encode(bytes: &[u8]) -> String {
	let mut s = String::with_capacity(bytes.len() * 2);
	for &b in bytes {
		s.push_str(&format!("{b:02x}"));
	}
	s
}
