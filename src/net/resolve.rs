use crate::config::DEFAULT_PORT;
use crate::error::{Error, Result};
use std::net::IpAddr;
use tokio::net::UdpSocket;

/// Resolved server address.
#[derive(Debug, Clone)]
pub struct ServerAddress {
	pub host: String,
	pub port: u16,
	pub srv: bool,
}

/// Parse a user-provided address string and optionally resolve SRV records.
pub async fn resolve(input: &str) -> Result<ServerAddress> {
	let input = input.trim();

	if let Some((host, port_str)) = split_host_port(input) {
		let port: u16 = port_str
			.parse()
			.map_err(|_| Error::InvalidAddress(format!("invalid port: {port_str}")))?;
		return Ok(ServerAddress {
			host: host.to_string(),
			port,
			srv: false,
		});
	}

	// No port specified — try SRV lookup if it looks like a domain
	if input.parse::<IpAddr>().is_err() {
		if let Some(addr) = srv_lookup(input).await {
			return Ok(addr);
		}
	}

	Ok(ServerAddress {
		host: input.to_string(),
		port: DEFAULT_PORT,
		srv: false,
	})
}

fn split_host_port(input: &str) -> Option<(&str, &str)> {
	if input.starts_with('[') {
		let end = input.find(']')?;
		let host = &input[1..end];
		let rest = &input[end + 1..];
		if let Some(port_str) = rest.strip_prefix(':') {
			return Some((host, port_str));
		}
		return None;
	}

	let colon_count = input.chars().filter(|&c| c == ':').count();
	if colon_count == 1 {
		let idx = input.rfind(':')?;
		return Some((&input[..idx], &input[idx + 1..]));
	}

	None
}

async fn srv_lookup(domain: &str) -> Option<ServerAddress> {
	let query = build_srv_query(domain)?;

	let socket = match UdpSocket::bind("0.0.0.0:0").await {
		Ok(s) => s,
		Err(_) => {
			tracing::debug!("SRV: failed to bind UDP socket");
			return None;
		}
	};

	if socket.send_to(&query, "8.8.8.8:53").await.is_err() {
		tracing::debug!("SRV: failed to send query to 8.8.8.8:53");
		return None;
	}

	let mut buf = [0u8; 512];
	let timeout = tokio::time::timeout(
		std::time::Duration::from_secs(3),
		socket.recv_from(&mut buf),
	)
	.await;

	let (len, _) = timeout.ok()?.ok()?;
	parse_srv_response(&buf[..len])
}

fn build_srv_query(domain: &str) -> Option<Vec<u8>> {
	let mut pkt = Vec::with_capacity(64);

	pkt.extend_from_slice(&[0xAB, 0xCD]);
	pkt.extend_from_slice(&[0x01, 0x00]);
	pkt.extend_from_slice(&[0x00, 0x01]);
	pkt.extend_from_slice(&[0x00, 0x00]);
	pkt.extend_from_slice(&[0x00, 0x00]);
	pkt.extend_from_slice(&[0x00, 0x00]);

	let labels: Vec<&str> = ["_minecraft", "_tcp"]
		.iter()
		.copied()
		.chain(domain.split('.'))
		.collect();

	for label in &labels {
		let bytes = label.as_bytes();
		if bytes.is_empty() || bytes.len() > 63 {
			return None;
		}
		pkt.push(bytes.len() as u8);
		pkt.extend_from_slice(bytes);
	}
	pkt.push(0x00);

	pkt.extend_from_slice(&[0x00, 0x21]);
	pkt.extend_from_slice(&[0x00, 0x01]);

	Some(pkt)
}

fn parse_srv_response(data: &[u8]) -> Option<ServerAddress> {
	if data.len() < 12 {
		return None;
	}

	let answer_count = u16::from_be_bytes([data[6], data[7]]) as usize;
	if answer_count == 0 {
		return None;
	}

	let mut pos = 12;
	pos = skip_dns_name(data, pos)?;
	pos += 4;

	pos = skip_dns_name(data, pos)?;
	if pos + 10 > data.len() {
		return None;
	}

	let rtype = u16::from_be_bytes([data[pos], data[pos + 1]]);
	pos += 2;
	pos += 2;
	pos += 4;
	let rdlength = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
	pos += 2;

	if rtype != 33 || pos + rdlength > data.len() || rdlength < 7 {
		return None;
	}

	let port = u16::from_be_bytes([data[pos + 4], data[pos + 5]]);
	let target = read_dns_name(data, pos + 6)?;

	let target = target.trim_end_matches('.');
	if target.is_empty() {
		return None;
	}

	Some(ServerAddress {
		host: target.to_string(),
		port,
		srv: true,
	})
}

fn skip_dns_name(data: &[u8], mut pos: usize) -> Option<usize> {
	loop {
		if pos >= data.len() {
			return None;
		}
		let len = data[pos] as usize;
		if len == 0 {
			return Some(pos + 1);
		}
		if len & 0xC0 == 0xC0 {
			return Some(pos + 2);
		}
		pos += 1 + len;
	}
}

fn read_dns_name(data: &[u8], mut pos: usize) -> Option<String> {
	let mut name = String::new();
	let mut jumps = 0;

	loop {
		if pos >= data.len() || jumps > 10 {
			return None;
		}
		let len = data[pos] as usize;
		if len == 0 {
			break;
		}
		if len & 0xC0 == 0xC0 {
			if pos + 1 >= data.len() {
				return None;
			}
			let offset = ((len & 0x3F) << 8) | data[pos + 1] as usize;
			pos = offset;
			jumps += 1;
			continue;
		}
		if pos + 1 + len > data.len() {
			return None;
		}
		if !name.is_empty() {
			name.push('.');
		}
		name.push_str(std::str::from_utf8(&data[pos + 1..pos + 1 + len]).ok()?);
		pos += 1 + len;
	}

	Some(name)
}
