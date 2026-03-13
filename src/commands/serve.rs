use crate::error::Result;
use crate::net::resolve;
use crate::protocol::{legacy, slp};
use crate::style;
use crate::types::{ChatComponent, Description};
use axum::{
	extract::{ConnectInfo, Path},
	http::{StatusCode, HeaderMap},
	response::IntoResponse,
	routing::get,
	Router,
	Json,
};
use dashmap::DashMap;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::time::timeout;

// ── mcstatus.io v2 structures ──────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct McStatusResponse {
	pub online: bool,
	pub host: String,
	pub port: u16,
	pub ip_address: Option<String>,
	pub eula_blocked: bool,
	pub retrieved_at: u64,
	pub expires_at: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version: Option<McVersion>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub players: Option<McPlayers>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub motd: Option<McMotd>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub icon: Option<String>,
	pub mods: Vec<McMod>,
	pub software: Option<String>,
	pub plugins: Vec<McPlugin>,
	pub srv_record: Option<McSrvRecord>,
}

#[derive(Debug, Serialize, Clone)]
pub struct McMod {
	pub name: String,
	pub version: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct McPlugin {
	pub name: String,
	pub version: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct McVersion {
	pub name_raw: String,
	pub name_clean: String,
	pub name_html: String,
	pub protocol: i32,
}

#[derive(Debug, Serialize, Clone)]
pub struct McPlayers {
	pub online: i32,
	pub max: i32,
	pub list: Vec<McPlayerSample>,
}

#[derive(Debug, Serialize, Clone)]
pub struct McPlayerSample {
	pub uuid: String,
	pub name_raw: String,
	pub name_clean: String,
	pub name_html: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct McMotd {
	pub raw: String,
	pub clean: String,
	pub html: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct McSrvRecord {
	pub host: String,
	pub port: u16,
}

// ── Server State ─────────────────────────────────────

struct AppState {
	cache: DashMap<String, (McStatusResponse, Instant)>,
	rate_limiter: DashMap<SocketAddr, (u32, Instant)>,
}

pub async fn run(host: &str, port: u16) -> Result<()> {
	let major_version = env!("CARGO_PKG_VERSION")
		.split('.')
		.next()
		.unwrap_or("1");
	let route = format!("/v{major_version}/status/java/{{address}}");

	let state = Arc::new(AppState {
		cache: DashMap::new(),
		rate_limiter: DashMap::new(),
	});

	let state_for_cleanup = state.clone();
	tokio::spawn(async move {
		loop {
			tokio::time::sleep(Duration::from_secs(60)).await;
			// Simple cleanup: clear everything every minute to prevent unbounded growth.
			// In a real production app, we'd use a TTL cache.
			state_for_cleanup.cache.clear();
			state_for_cleanup.rate_limiter.clear();
			tracing::debug!("ZeroTick serve: cleared cache and rate limiter");
		}
	});

	let app = Router::new()
		.route(&route.replace("{address}", ":address"), get(handle_java_status))
		.with_state(state);

	let addr = format!("{}:{}", host, port);
	let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
		crate::error::Error::Protocol(format!("failed to bind to {addr}: {e}"))
	})?;

	println!("{}", style::success(&format!("ZeroTick serve listening on http://{}", addr)));

	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
		.await
		.map_err(|e| crate::error::Error::Protocol(format!("server error: {e}")))?;

	Ok(())
}

async fn handle_java_status(
	Path(address): Path<String>,
	ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
	axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
	let start_time = Instant::now();

	// Rate limiting: 5 requests per second per IP
	{
		let ip = client_addr.ip();
		let mut entry = state.rate_limiter.entry(SocketAddr::new(ip, 0)).or_insert((0, Instant::now()));
		let val = entry.value_mut();
		if val.1.elapsed() > Duration::from_secs(1) {
			val.0 = 0;
			val.1 = Instant::now();
		}
		if val.0 >= 5 {
			log_request(&address, client_addr, StatusCode::TOO_MANY_REQUESTS, start_time.elapsed(), false);
			return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({ "error": "Too many requests" }))).into_response();
		}
		val.0 += 1;
	}

	// Caching: 30 seconds
	if let Some(cached) = state.cache.get(&address) {
		let (response, timestamp) = cached.value();
		if timestamp.elapsed() < Duration::from_secs(30) {
			log_request(&address, client_addr, StatusCode::OK, start_time.elapsed(), true);
			let mut headers = HeaderMap::new();
			headers.insert("X-Cache-Hit", "true".parse().unwrap());
			let remaining = 30u64.saturating_sub(timestamp.elapsed().as_secs());
			headers.insert("X-Cache-Time-Remaining", remaining.to_string().parse().unwrap());
			return (headers, Json(response.clone())).into_response();
		}
	}

	// Fetch status
	let response = match fetch_status(&address).await {
		Ok(resp) => resp,
		Err(e) => {
			log_request(&address, client_addr, StatusCode::INTERNAL_SERVER_ERROR, start_time.elapsed(), false);
			return (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {e}")).into_response();
		}
	};

	state.cache.insert(address.clone(), (response.clone(), Instant::now()));

	log_request(&address, client_addr, StatusCode::OK, start_time.elapsed(), false);
	(StatusCode::OK, Json(response)).into_response()
}

async fn fetch_status(address: &str) -> Result<McStatusResponse> {
	let resolved = resolve::resolve(address).await?;
	let connect_timeout = Duration::from_secs(5);
	let socket_addr = format!("{}:{}", resolved.host, resolved.port);

	let ip_address = tokio::net::lookup_host(&socket_addr)
		.await
		.ok()
		.and_then(|mut addrs| addrs.next())
		.map(|addr| addr.ip().to_string());

	let retrieved_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
	let expires_at = retrieved_at + 30000;

	let srv_record = if resolved.srv {
		Some(McSrvRecord {
			host: resolved.host.clone(),
			port: resolved.port,
		})
	} else {
		None
	};

	let mut stream = match timeout(connect_timeout, TcpStream::connect(&socket_addr)).await {
		Ok(Ok(s)) => s,
		_ => {
			return Ok(offline_response(resolved.host, resolved.port, ip_address, retrieved_at, expires_at, srv_record));
		}
	};

	let (status, _latency) = match slp::ping(&mut stream, &resolved.host, resolved.port).await {
		Ok(res) => res,
		Err(_) => {
			// Try legacy
			let mut stream2 = match timeout(connect_timeout, TcpStream::connect(&socket_addr)).await {
				Ok(Ok(s)) => s,
				_ => return Ok(offline_response(resolved.host.clone(), resolved.port, ip_address, retrieved_at, expires_at, srv_record)),
			};
			match legacy::ping(&mut stream2, &resolved.host, resolved.port).await {
				Ok(s) => (s, Duration::ZERO),
				Err(_) => return Ok(offline_response(resolved.host, resolved.port, ip_address, retrieved_at, expires_at, srv_record)),
			}
		}
	};

	let eula_blocked = crate::mojang::blocked::is_blocked(&resolved.host, resolved.port).await.unwrap_or(false);

	Ok(McStatusResponse {
		online: true,
		host: resolved.host,
		port: resolved.port,
		ip_address,
		eula_blocked,
		retrieved_at,
		expires_at,
		version: Some(McVersion {
			name_raw: status.version.name.clone(),
			name_clean: clean_motd(&status.version.name),
			name_html: motd_to_html(&status.version.name),
			protocol: status.version.protocol,
		}),
		players: Some(McPlayers {
			online: status.players.online,
			max: status.players.max,
			list: status.players.sample.iter().map(|p| McPlayerSample {
				uuid: p.id.clone(),
				name_raw: p.name.clone(),
				name_clean: clean_motd(&p.name),
				name_html: motd_to_html(&p.name),
			}).collect(),
		}),
		motd: Some(McMotd {
			raw: raw_motd(&status.description),
			clean: clean_description(&status.description),
			html: description_to_html(&status.description),
		}),
		icon: status.favicon,
		mods: Vec::new(),
		software: None,
		plugins: Vec::new(),
		srv_record,
	})
}

fn offline_response(host: String, port: u16, ip_address: Option<String>, retrieved_at: u64, expires_at: u64, srv_record: Option<McSrvRecord>) -> McStatusResponse {
	McStatusResponse {
		online: false,
		host,
		port,
		ip_address,
		eula_blocked: false,
		retrieved_at,
		expires_at,
		version: None,
		players: None,
		motd: None,
		icon: None,
		mods: Vec::new(),
		software: None,
		plugins: Vec::new(),
		srv_record,
	}
}

// ── MOTD Utilities ───────────────────────────────────

fn raw_motd(desc: &Description) -> String {
	match desc {
		Description::Plain(s) => s.clone(),
		Description::Component(c) => component_to_raw(c),
	}
}

fn component_to_raw(comp: &ChatComponent) -> String {
	let mut out = comp.text.clone();
	for child in &comp.extra {
		out.push_str(&component_to_raw(child));
	}
	out
}

fn clean_description(desc: &Description) -> String {
	clean_motd(&raw_motd(desc))
}

fn clean_motd(input: &str) -> String {
	let mut out = String::new();
	let mut chars = input.chars().peekable();
	while let Some(ch) = chars.next() {
		if ch == '\u{00A7}' {
			chars.next();
		} else {
			out.push(ch);
		}
	}
	out
}

fn description_to_html(desc: &Description) -> String {
	match desc {
		Description::Plain(s) => motd_to_html(s),
		Description::Component(c) => component_to_html(c),
	}
}

fn motd_to_html(input: &str) -> String {
	let mut out = String::from("<span>");
	let mut chars = input.chars().peekable();

	let mut color: Option<&str> = None;
	let mut bold = false;
	let mut strike = false;
	let mut underline = false;
	let mut italic = false;

	while let Some(ch) = chars.next() {
		if ch == '\u{00A7}' {
			if let Some(code) = chars.next() {
				match code {
					'0' => color = Some("#000000"),
					'1' => color = Some("#0000AA"),
					'2' => color = Some("#00AA00"),
					'3' => color = Some("#00AAAA"),
					'4' => color = Some("#AA0000"),
					'5' => color = Some("#AA00AA"),
					'6' => color = Some("#FFAA00"),
					'7' => color = Some("#AAAAAA"),
					'8' => color = Some("#555555"),
					'9' => color = Some("#5555FF"),
					'a' => color = Some("#55FF55"),
					'b' => color = Some("#55FFFF"),
					'c' => color = Some("#FF5555"),
					'd' => color = Some("#FF55FF"),
					'e' => color = Some("#FFFF55"),
					'f' => color = Some("#FFFFFF"),
					'l' => bold = true,
					'm' => strike = true,
					'n' => underline = true,
					'o' => italic = true,
					'r' => {
						color = None;
						bold = false;
						strike = false;
						underline = false;
						italic = false;
					}
					_ => {}
				}

				out.push_str("</span><span style=\"");
				if let Some(c) = color {
					out.push_str(&format!("color: {};", c));
				}
				if bold {
					out.push_str("font-weight: bold;");
				}
				if strike {
					out.push_str("text-decoration: line-through;");
				}
				if underline {
					out.push_str("text-decoration: underline;");
				}
				if italic {
					out.push_str("font-style: italic;");
				}
				out.push_str("\">");
			}
		} else {
			match ch {
				'<' => out.push_str("&lt;"),
				'>' => out.push_str("&gt;"),
				'&' => out.push_str("&amp;"),
				'\n' => out.push_str("<br>"),
				_ => out.push(ch),
			}
		}
	}
	out.push_str("</span>");
	out
}

fn component_to_html(comp: &ChatComponent) -> String {
	let mut style = String::new();
	if let Some(ref color) = comp.color {
		let css_color = if color.starts_with('#') {
			color.clone()
		} else {
			match color.as_str() {
				"black" => "#000000".to_string(),
				"dark_blue" => "#0000AA".to_string(),
				"dark_green" => "#00AA00".to_string(),
				"dark_aqua" => "#00AAAA".to_string(),
				"dark_red" => "#AA0000".to_string(),
				"dark_purple" => "#AA00AA".to_string(),
				"gold" => "#FFAA00".to_string(),
				"gray" => "#AAAAAA".to_string(),
				"dark_gray" => "#555555".to_string(),
				"blue" => "#5555FF".to_string(),
				"green" => "#55FF55".to_string(),
				"aqua" => "#55FFFF".to_string(),
				"red" => "#FF5555".to_string(),
				"light_purple" => "#FF55FF".to_string(),
				"yellow" => "#FFFF55".to_string(),
				"white" => "#FFFFFF".to_string(),
				_ => color.clone(),
			}
		};
		style.push_str(&format!("color: {};", css_color));
	}
	if comp.bold == Some(true) {
		style.push_str("font-weight: bold;");
	}
	if comp.italic == Some(true) {
		style.push_str("font-style: italic;");
	}
	if comp.underlined == Some(true) {
		style.push_str("text-decoration: underline;");
	}
	if comp.strikethrough == Some(true) {
		style.push_str("text-decoration: line-through;");
	}

	let escaped_text = comp
		.text
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('\n', "<br>");

	let mut out = format!("<span style=\"{}\">{}", style, escaped_text);
	for child in &comp.extra {
		out.push_str(&component_to_html(child));
	}
	out.push_str("</span>");
	out
}

// ── Logging ──────────────────────────────────────────

fn log_request(address: &str, client_addr: SocketAddr, status: StatusCode, duration: Duration, cached: bool) {
	use colored::Colorize;
	let status_str = if status.is_success() {
		status.as_str().green()
	} else if status.is_client_error() {
		status.as_str().yellow()
	} else {
		status.as_str().red()
	};

	let cache_str = if cached {
		"[CACHE]".cyan()
	} else {
		"[MISS]".dimmed()
	};

	println!(
		"{} {} {} {} {} {}",
		style::BULLET.cyan(),
		client_addr.to_string().dimmed(),
		"GET".bold(),
		address.color(style::ACCENT),
		status_str,
		format!("{:?}", duration).dimmed(),
	);
	println!("  {} {}", style::DOT.dimmed(), cache_str);
}
