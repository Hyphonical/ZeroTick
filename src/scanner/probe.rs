use crate::protocol::slp;
use crate::types::ServerStatus;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Result of a single probe attempt.
#[derive(Debug, Clone)]
pub struct ProbeResult {
	pub addr: Ipv4Addr,
	pub port: u16,
	pub status: ServerStatus,
	pub latency: Duration,
}

/// Attempt to connect and SLP-ping a single address.
pub async fn probe(
	addr: Ipv4Addr,
	port: u16,
	connect_timeout: Duration,
	read_timeout: Duration,
) -> Option<ProbeResult> {
	let socket_addr = std::net::SocketAddr::new(addr.into(), port);

	let mut stream = match timeout(connect_timeout, TcpStream::connect(socket_addr)).await {
		Ok(Ok(s)) => s,
		_ => return None,
	};

	let host = addr.to_string();
	match timeout(read_timeout, slp::ping(&mut stream, &host, port)).await {
		Ok(Ok((status, latency))) => Some(ProbeResult {
			addr,
			port,
			status,
			latency,
		}),
		Ok(Err(e)) => {
			tracing::trace!("SLP failed for {addr}:{port}: {e}");
			None
		}
		Err(_) => {
			tracing::trace!("SLP timed out for {addr}:{port}");
			None
		}
	}
}
