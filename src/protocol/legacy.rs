use crate::error::{Error, Result};
use crate::types::{Description, Players, ServerStatus, Version};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Perform a legacy Server List Ping (≤1.6) using the `0xFE 0x01` protocol.
pub async fn ping(stream: &mut TcpStream, host: &str, port: u16) -> Result<ServerStatus> {
	let mut buf: Vec<u8> = Vec::with_capacity(64);
	buf.push(0xFE);
	buf.push(0x01);
	buf.push(0xFA);

	let channel = "MC|PingHost";
	let channel_utf16: Vec<u16> = channel.encode_utf16().collect();
	buf.extend_from_slice(&(channel_utf16.len() as u16).to_be_bytes());
	for cp in &channel_utf16 {
		buf.extend_from_slice(&cp.to_be_bytes());
	}

	let host_utf16: Vec<u16> = host.encode_utf16().collect();
	let data_len = 7 + (host_utf16.len() * 2);
	buf.extend_from_slice(&(data_len as u16).to_be_bytes());
	buf.push(0x4A);
	buf.extend_from_slice(&(host_utf16.len() as u16).to_be_bytes());
	for cp in &host_utf16 {
		buf.extend_from_slice(&cp.to_be_bytes());
	}
	buf.extend_from_slice(&(port as u32).to_be_bytes());

	stream.write_all(&buf).await?;
	stream.flush().await?;

	let id = stream.read_u8().await?;
	if id != 0xFF {
		return Err(Error::protocol(format!(
			"expected legacy kick (0xFF), got 0x{id:02X}"
		)));
	}

	let str_len = stream.read_u16().await? as usize;
	let mut raw = vec![0u8; str_len * 2];
	stream.read_exact(&mut raw).await?;

	let chars: Vec<u16> = raw
		.chunks_exact(2)
		.map(|c| u16::from_be_bytes([c[0], c[1]]))
		.collect();
	let response = String::from_utf16_lossy(&chars);

	parse_legacy_response(&response)
}

fn parse_legacy_response(s: &str) -> Result<ServerStatus> {
	if s.starts_with("§1\0") {
		let parts: Vec<&str> = s.splitn(6, '\0').collect();
		if parts.len() < 6 {
			return Err(Error::protocol("incomplete legacy response"));
		}

		let protocol: i32 = parts[1].parse().unwrap_or(0);
		let version_name = parts[2].to_string();
		let motd = parts[3].to_string();
		let online: i32 = parts[4].parse().unwrap_or(0);
		let max: i32 = parts[5].parse().unwrap_or(0);

		Ok(ServerStatus {
			version: Version {
				name: version_name,
				protocol,
			},
			players: Players {
				max,
				online,
				sample: Vec::new(),
			},
			description: Description::Plain(motd),
			favicon: None,
			enforces_secure_chat: None,
			previews_chat: None,
			forge_data: None,
			mod_info: None,
		})
	} else {
		let parts: Vec<&str> = s.rsplitn(3, '§').collect();
		if parts.len() < 3 {
			return Err(Error::protocol("incomplete old legacy response"));
		}

		let max: i32 = parts[0].parse().unwrap_or(0);
		let online: i32 = parts[1].parse().unwrap_or(0);
		let motd = parts[2].to_string();

		Ok(ServerStatus {
			version: Version {
				name: "≤1.3".to_string(),
				protocol: 0,
			},
			players: Players {
				max,
				online,
				sample: Vec::new(),
			},
			description: Description::Plain(motd),
			favicon: None,
			enforces_secure_chat: None,
			previews_chat: None,
			forge_data: None,
			mod_info: None,
		})
	}
}
