use crate::error::{Error, Result};
use rustls::ClientConfig;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// Minimal async HTTPS GET — returns the response body as a string.
///
/// Only supports simple `https://host/path` URLs with `Content-Length` responses.
pub async fn get(url: &str) -> Result<String> {
	let (host, path) = parse_url(url)?;

	// TLS config with system roots
	let mut root_store = rustls::RootCertStore::empty();
	root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

	let config = ClientConfig::builder()
		.with_root_certificates(root_store)
		.with_no_client_auth();

	let connector = TlsConnector::from(Arc::new(config));
	let server_name = host
		.to_string()
		.try_into()
		.map_err(|_| Error::MojangApi(format!("invalid hostname: {host}")))?;

	let tcp = TcpStream::connect((host.as_str(), 443)).await?;
	let mut tls = connector.connect(server_name, tcp).await?;

	// Send HTTP/1.1 GET
	let request = format!(
		"GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nUser-Agent: zerotick\r\n\r\n"
	);
	tls.write_all(request.as_bytes()).await?;
	tls.flush().await?;

	// Read entire response
	let mut response = Vec::new();
	tls.read_to_end(&mut response).await?;

	let response_str = String::from_utf8_lossy(&response);

	// Split headers from body
	let body_start = response_str
		.find("\r\n\r\n")
		.ok_or_else(|| Error::MojangApi("malformed HTTP response".into()))?;

	let headers = &response_str[..body_start];
	let body = &response_str[body_start + 4..];

	// Check status line
	let status_line = headers.lines().next().unwrap_or("");
	if !status_line.contains("200") {
		return Err(Error::MojangApi(format!("HTTP {status_line}")));
	}

	// Handle chunked transfer encoding
	if headers
		.to_lowercase()
		.contains("transfer-encoding: chunked")
	{
		return Ok(decode_chunked(body));
	}

	Ok(body.to_string())
}

/// Parse `https://host/path` into `(host, path)`.
fn parse_url(url: &str) -> Result<(String, String)> {
	let url = url
		.strip_prefix("https://")
		.ok_or_else(|| Error::MojangApi(format!("not HTTPS: {url}")))?;

	let (host, path) = match url.find('/') {
		Some(i) => (url[..i].to_string(), url[i..].to_string()),
		None => (url.to_string(), "/".to_string()),
	};

	Ok((host, path))
}

/// Decode a chunked transfer-encoded body.
fn decode_chunked(body: &str) -> String {
	let mut result = String::new();
	let mut remaining = body;

	loop {
		let remaining_trimmed = remaining.trim_start_matches("\r\n");
		let line_end = match remaining_trimmed.find("\r\n") {
			Some(i) => i,
			None => break,
		};

		let size_str = &remaining_trimmed[..line_end];
		let size = match usize::from_str_radix(size_str.trim(), 16) {
			Ok(s) => s,
			Err(_) => break,
		};

		if size == 0 {
			break;
		}

		let chunk_start = line_end + 2;
		if chunk_start + size <= remaining_trimmed.len() {
			result.push_str(&remaining_trimmed[chunk_start..chunk_start + size]);
			remaining = &remaining_trimmed[chunk_start + size..];
		} else {
			// Incomplete chunk, take what we can
			result.push_str(&remaining_trimmed[chunk_start..]);
			break;
		}
	}

	result
}
