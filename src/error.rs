use std::time::Duration;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{0}")]
	Io(#[from] std::io::Error),

	#[error("varint exceeds 5-byte limit")]
	VarIntTooLong,

	#[error("packet too large: {0} bytes")]
	PacketTooLarge(usize),

	#[error("protocol error: {0}")]
	Protocol(String),

	#[error("{0}")]
	Json(#[from] serde_json::Error),

	/// Not yet used; reserved for DNS resolution failures.
	#[error("dns: {0}")]
	#[allow(dead_code)]
	Dns(String),

	#[error("{0}")]
	Tls(#[from] rustls::Error),

	/// Not yet used; reserved for connection timeout errors.
	#[error("timed out after {0:?}")]
	#[allow(dead_code)]
	Timeout(Duration),

	#[error("mojang api: {0}")]
	MojangApi(String),

	#[error("{0}")]
	InvalidAddress(String),
}

impl Error {
	pub fn protocol(msg: impl Into<String>) -> Self {
		Self::Protocol(msg.into())
	}

	/// Create a DNS error; reserved for future DNS resolution failures.
	#[allow(dead_code)]
	pub fn dns(msg: impl Into<String>) -> Self {
		Self::Dns(msg.into())
	}

	/// Create a timeout error; reserved for connection timeout handling.
	#[allow(dead_code)]
	pub fn timeout(duration: Duration) -> Self {
		Self::Timeout(duration)
	}
}
