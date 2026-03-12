use crate::config::MAX_PROFILE_LOOKUPS;
use crate::error::Result;
use crate::mojang;
use crate::net::resolve;
use crate::protocol::{legacy, slp};
use crate::style;
use crate::style::motd;
use crate::style::tree::Tree;
use crate::types::ServerInfo;
use colored::Colorize;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub async fn run(address: &str, timeout_ms: u64) -> Result<()> {
	let connect_timeout = Duration::from_millis(timeout_ms);

	// ── Resolve ──────────────────────────────────────
	let resolved = resolve::resolve(address).await?;

	if resolved.srv {
		tracing::info!("SRV resolved to {}:{}", resolved.host, resolved.port);
	}

	let display_addr = format!("{}:{}", resolved.host, resolved.port);
	println!("{}", style::header(&display_addr));

	if resolved.srv {
		println!(
			"{}",
			style::detail("SRV", &format!("→ {}:{}", resolved.host, resolved.port))
		);
	}

	// ── Connect ──────────────────────────────────────
	let socket_addr = format!("{}:{}", resolved.host, resolved.port);
	let mut stream = match timeout(connect_timeout, TcpStream::connect(&socket_addr)).await {
		Ok(Ok(s)) => s,
		Ok(Err(e)) => {
			println!("{}", style::error(&format!("connection failed: {e}")));
			return Ok(());
		}
		Err(_) => {
			println!(
				"{}",
				style::error(&format!("connection timed out ({timeout_ms}ms)"))
			);
			return Ok(());
		}
	};

	// ── SLP ──────────────────────────────────────────
	let (status, latency) = match slp::ping(&mut stream, &resolved.host, resolved.port).await {
		Ok(result) => result,
		Err(e) => {
			tracing::debug!("modern SLP failed: {e}, trying legacy...");
			let mut stream2 = match timeout(connect_timeout, TcpStream::connect(&socket_addr)).await
			{
				Ok(Ok(s)) => s,
				_ => {
					println!("{}", style::error(&format!("SLP failed: {e}")));
					return Ok(());
				}
			};
			match legacy::ping(&mut stream2, &resolved.host, resolved.port).await {
				Ok(status) => (status, Duration::ZERO),
				Err(e2) => {
					println!(
						"{}",
						style::error(&format!("SLP failed: {e} / legacy: {e2}"))
					);
					return Ok(());
				}
			}
		}
	};

	// ── Blocked check (non-fatal) ────────────────────
	let blocked = match mojang::blocked::is_blocked(&resolved.host, resolved.port).await {
		Ok(b) => Some(b),
		Err(e) => {
			tracing::warn!("blocked check failed: {e}");
			None
		}
	};

	// ── Build ServerInfo ─────────────────────────────
	let info = ServerInfo {
		address: resolved.host.clone(),
		port: resolved.port,
		status,
		latency,
		blocked,
		srv: resolved.srv,
	};

	// ── Player profile lookups (non-fatal) ───────────
	let profiles = fetch_profiles(&info).await;

	// ── Render ───────────────────────────────────────
	render_status(&info, &profiles);

	Ok(())
}

/// Fetch Mojang profiles for the first N players in the sample list.
async fn fetch_profiles(
	info: &ServerInfo,
) -> Vec<(String, Option<mojang::profile::PlayerProfile>)> {
	let mut results = Vec::new();

	for player in info.status.players.sample.iter().take(MAX_PROFILE_LOOKUPS) {
		// Skip fake/zero UUIDs that some servers use
		let clean: String = player.id.chars().filter(|c| *c != '-').collect();
		if clean.chars().all(|c| c == '0') || clean.len() != 32 {
			results.push((player.name.clone(), None));
			continue;
		}

		match mojang::profile::fetch(&player.id).await {
			Ok(profile) => {
				tracing::debug!("fetched profile for {}: {:?}", player.name, profile);
				results.push((player.name.clone(), Some(profile)));
			}
			Err(e) => {
				tracing::debug!("profile fetch failed for {}: {e}", player.name);
				results.push((player.name.clone(), None));
			}
		}
	}

	results
}

fn render_status(info: &ServerInfo, profiles: &[(String, Option<mojang::profile::PlayerProfile>)]) {
	let status = &info.status;
	let mut tree = Tree::new();

	// Version
	tree.add(
		"Version",
		format!(
			"{} {}",
			status.version.name.white().bold(),
			format!("({})", status.version.protocol).color(style::MUTED)
		),
	);

	// Latency
	if info.latency > Duration::ZERO {
		let ms = info.latency.as_millis();
		let styled = if ms < 50 {
			format!("{ms}ms").green().bold().to_string()
		} else if ms < 150 {
			format!("{ms}ms").yellow().bold().to_string()
		} else {
			format!("{ms}ms").red().bold().to_string()
		};
		tree.add("Latency", styled);
	}

	// Players
	let players_str = format!(
		"{}/{}",
		style::value(format_number(status.players.online)),
		format_number(status.players.max)
	);

	if status.players.sample.is_empty() {
		tree.add("Players", players_str);
	} else {
		let max_show = 8usize;
		let mut children: Vec<(String, String)> = Vec::new();

		for (_i, player) in status.players.sample.iter().enumerate().take(max_show) {
			// Check if we have a profile for this player
			let profile_info = profiles.iter().find(|(name, _)| name == &player.name);

			let mut line = player.name.color(style::ACCENT).to_string();

			if let Some((_, Some(profile))) = profile_info {
				if profile.skin_url.is_some() || profile.cape_url.is_some() {
					let mut extras = Vec::new();
					if profile.skin_url.is_some() {
						extras.push("skin");
					}
					if profile.cape_url.is_some() {
						extras.push("cape");
					}
					line.push_str(&format!(
						" {}",
						format!("[{}]", extras.join(", ")).color(style::MUTED)
					));
				}
			}

			children.push((String::new(), line));
		}

		let remaining = status.players.sample.len().saturating_sub(max_show);
		if remaining > 0 {
			children.push((
				String::new(),
				format!("… {} more", remaining).dimmed().to_string(),
			));
		}

		tree.add_with_children("Players", players_str, children);
	}

	// MOTD
	let motd_rendered = motd::render(&status.description);
	let motd_lines: Vec<&str> = motd_rendered.lines().collect();
	if let Some(first) = motd_lines.first() {
		if motd_lines.len() == 1 {
			tree.add("MOTD", first.to_string());
		} else {
			let rest = motd_lines[1..]
				.iter()
				.map(|l| (String::new(), l.to_string()))
				.collect();
			tree.add_with_children("MOTD", first.to_string(), rest);
		}
	}

	// Secure chat
	if let Some(enforced) = status.enforces_secure_chat {
		let val = if enforced {
			"enforced".green().to_string()
		} else {
			"not enforced".red().to_string()
		};
		tree.add("Secure", val);
	}

	// Previews chat
	if let Some(previews) = status.previews_chat {
		let val = if previews {
			"enabled".green().to_string()
		} else {
			"disabled".color(style::MUTED).to_string()
		};
		tree.add("Preview", val);
	}

	// Blocked
	if let Some(is_blocked) = info.blocked {
		let val = if is_blocked {
			"Yes".red().bold().to_string()
		} else {
			"No".green().to_string()
		};
		tree.add("Blocked", val);
	}

	// Favicon
	if let Some(ref favicon) = status.favicon {
		let data_len = favicon
			.find(',')
			.map(|i| favicon.len() - i - 1)
			.unwrap_or(favicon.len());
		tree.add(
			"Favicon",
			format!("64×64 embedded (~{} bytes)", data_len)
				.dimmed()
				.to_string(),
		);
	}

	// Forge/Mods
	if status.forge_data.is_some() || status.mod_info.is_some() {
		tree.add(
			"Mods",
			"Forge/modded server detected"
				.color(style::INFO)
				.to_string(),
		);
	}

	print!("{}", tree.render(10));
}

fn format_number(n: i32) -> String {
	let s = n.to_string();
	let bytes = s.as_bytes();
	let mut result = String::new();
	for (i, &b) in bytes.iter().enumerate() {
		if i > 0 && (bytes.len() - i).is_multiple_of(3) {
			result.push(',');
		}
		result.push(b as char);
	}
	result
}
