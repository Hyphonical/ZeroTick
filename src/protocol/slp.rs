use crate::config::PROTOCOL_VERSION;
use crate::error::{Error, Result};
use crate::protocol::packet::Packet;
use crate::types::ServerStatus;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

/// Perform a modern Server List Ping (1.7+) and return the status + latency.
pub async fn ping(
	stream: &mut TcpStream,
	host: &str,
	port: u16,
) -> Result<(ServerStatus, Duration)> {
	// ── Handshake (packet 0x00) ──────────────────────
	let mut handshake = Packet::new(0x00);
	handshake
		.write_varint(PROTOCOL_VERSION)
		.write_string(host)
		.write_u16(port)
		.write_varint(1); // Next state: Status

	handshake.send(stream).await?;

	// ── Status Request (packet 0x00, empty) ──────────
	Packet::new(0x00).send(stream).await?;

	// ── Status Response ──────────────────────────────
	let response = Packet::recv(stream).await?;

	if response.id != 0x00 {
		return Err(Error::protocol(format!(
			"expected status response (0x00), got 0x{:02X}",
			response.id
		)));
	}

	let mut reader = response.reader();
	let json_str = reader.read_string()?;

	tracing::debug!("SLP JSON ({} bytes): {}", json_str.len(), json_str);

	let status: ServerStatus = serde_json::from_str(json_str)?;

	// ── Ping / Pong for latency ──────────────────────
	let payload: i64 = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default()
		.as_millis() as i64;

	let mut ping_packet = Packet::new(0x01);
	ping_packet.write_i64(payload);

	let start = Instant::now();
	ping_packet.send(stream).await?;

	let pong = Packet::recv(stream).await?;
	let latency = start.elapsed();

	if pong.id != 0x01 {
		tracing::warn!("expected pong (0x01), got 0x{:02X}", pong.id);
	}

	Ok((status, latency))
}
