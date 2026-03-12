
# ZeroTick ⚡

**Minecraft server reconnaissance that doesn't mess around.**

Query any server's status or scan entire IP ranges for Minecraft servers. No Java, no Minecraft client, no bloated dependencies. Just a single binary, raw protocol packets, and styled terminal output.

## Table of Contents

- [What's This About?](#whats-this-about)
- [Why Does This Exist?](#why-does-this-exist)
- [Features](#features-)
- [Quick Start](#quick-start-)
- [Usage](#usage)
  - [`status` - Query a Server](#status---query-a-server)
  - [`scan` - Scan IP Ranges](#scan---scan-ip-ranges)
  - [Global Options](#global-options)
- [How It Works](#how-it-works-)
- [Contributing](#contributing)
- [License](#license)

## What's This About?

You know when you vaguely remember the IP of a Minecraft server you played on three years ago, or you want to see what's running on your local network, or you just want to check if Hypixel is having a moment? That's ZeroTick.

Type `zerotick status mc.hypixel.net` and you get version, player count, MOTD, latency, secure chat status, Mojang blocked status — the works. All rendered in a clean tree layout with proper Minecraft color codes. Or point `zerotick scan` at a CIDR range and find every Minecraft server hiding on a subnet in seconds.

ZeroTick speaks the Minecraft protocol natively — both the modern Server List Ping (1.7+) and the legacy protocol (≤1.6). It resolves SRV records, checks Mojang's blocked server list, and even looks up player skin URLs. All from a ~3MB binary.

## Why Does This Exist?

There are other Minecraft ping tools out there. Most of them are Python scripts that take 4 seconds to query a single server, or web apps that route your requests through someone else's infrastructure, or Java programs that somehow need 200MB of RAM to send two packets.

I wanted something **instant**, **offline**, and **unix-philosophy friendly**. Something that starts up in milliseconds, gives me exactly the information I need, and doesn't phone home. Also, implementing the Minecraft protocol from scratch in Rust sounded like a fun weekend project. (It was not a weekend project.)

The scanner specifically exists because sometimes you inherit a network and need to know "is anyone running unauthorized game servers on this subnet?" The answer is usually yes. Now you can find them in under a minute.

## Features 🎯

- **🏓 Server List Ping**: Full modern SLP (1.7+) with JSON status, latency measurement, and ping/pong
- **👴 Legacy support**: Falls back to the ≤1.6 protocol for ancient servers
- **🔍 Mass scanning**: Scan entire CIDR ranges with configurable concurrency and rate limiting
- **🌐 SRV resolution**: Automatically resolves `_minecraft._tcp` DNS SRV records
- **🎨 MOTD rendering**: Full Minecraft chat component parsing with `§` codes and hex colors
- **🚫 Blocked check**: Checks addresses against Mojang's SHA-1 blocked servers list
- **👤 Player profiles**: Looks up skin/cape URLs for players in the sample list
- **🔒 Secure chat**: Shows whether the server enforces secure chat
- **⚡ Rate limiting**: Token-bucket rate limiter to avoid melting your router
- **📊 Styled output**: Clean tree and table renderers with a consistent color palette
- **📈 Progress bars**: Real-time scanning progress with live discovery count
- **🔇 Clean stderr/stdout**: Logs go to stderr, styled output to stdout — pipe-friendly
- **🦀 Single binary**: No runtime dependencies, no JVM, no Python, no Docker

## Quick Start 🚀

### Install

```bash
git clone https://github.com/yourusername/ZeroTick.git
cd ZeroTick
cargo build --release
```

Binary at `target/release/zerotick` (or `zerotick.exe` on Windows).

### Query a Server

```bash
zerotick status mc.hypixel.net
```

```
● mc.hypixel.net:25565
 ├─ Version    1.8-1.21.4 (769)
 ├─ Latency    23ms
 ├─ Players    42,069/100,000
 │   ├─ Technoblade
 │   ├─ Dream
 │   ╰─ … 42,067 more
 ├─ MOTD       Hypixel Network
 │              SUMMER EVENT + 50% OFF SALE
 ├─ Secure     enforced
 ├─ Blocked    no
 ╰─ Favicon    64×64 embedded (~5,432 bytes)
```

### Scan a Subnet

```bash
zerotick scan 192.168.1.0/24
```

```
● Scan: 192.168.1.0/24:25565  (256 addresses)
  · Rate       1000/s
  · Timeout    3000ms

  ━━━━━━━━━━━━━━━━━━━━╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌  256/256 00:00:04 elapsed

✓ 3 servers found

╭──────────────────┬──────────┬─────────┬────────╮
│ Address          │ Version  │ Players │ Ping   │
├──────────────────┼──────────┼─────────┼────────┤
│ 192.168.1.10     │ 1.21.4   │ 3/20    │ 2ms    │
│ 192.168.1.47     │ 1.20.4   │ 0/100   │ 5ms    │
│ 192.168.1.201    │ 1.19.2   │ 12/50   │ 8ms    │
╰──────────────────┴──────────┴─────────┴────────╯
```

## Usage

### `status` - Query a Server

```bash
zerotick status <ADDRESS> [OPTIONS]

Arguments:
  <ADDRESS>                     Server address (ip, ip:port, or domain)

Options:
  -t, --timeout <MS>            Connection timeout in milliseconds [default: 5000]
```

Queries a single Minecraft server and displays everything it knows. Tries the modern SLP protocol first, falls back to legacy if that fails. Resolves SRV records automatically for domains. Checks Mojang's blocked servers list. Looks up player profiles for skin/cape info.

**Examples:**

```bash
# Domain with default port
zerotick status mc.hypixel.net

# Explicit port
zerotick status play.example.com:25566

# Direct IP
zerotick status 192.168.1.100

# Short timeout for quick checks
zerotick status some.server.com -t 2000
```

> [!TIP]
> You don't need to specify `:25565` — ZeroTick uses the default Minecraft port automatically and checks SRV records for domains.

### `scan` - Scan IP Ranges

```bash
zerotick scan <RANGE> [OPTIONS]

Arguments:
  <RANGE>                       CIDR range (e.g. 192.168.1.0/24)

Options:
  -p, --port <PORT>             Target port [default: 25565]
  -c, --concurrency <N>         Maximum concurrent connections [default: 256]
  -t, --timeout <MS>            Connection timeout in milliseconds [default: 3000]
  --rate <N>                    Maximum new connections per second [default: 1000]
```

Scans an entire IP range for Minecraft servers. Addresses are probed in **randomized order** to avoid sequential scanning patterns. Uses a semaphore for concurrency control and a token-bucket rate limiter so you don't accidentally DoS your own network.

**Examples:**

```bash
# Scan a /24 subnet
zerotick scan 192.168.1.0/24

# Non-standard port
zerotick scan 10.0.0.0/16 -p 25566

# Aggressive scan (local network)
zerotick scan 192.168.0.0/16 -c 512 --rate 5000 -t 1000

# Gentle scan (remote, don't get banned)
zerotick scan 203.0.113.0/24 -c 32 --rate 100 -t 5000
```

> [!CAUTION]
> Scanning networks you don't own or have permission to scan may violate laws and terms of service. ZeroTick is a network reconnaissance tool — use it responsibly and only on networks where you have authorization.

> [!NOTE]
> The maximum supported range is `/8` (16M addresses). For anything larger, break it into multiple scans. A `/24` (256 addresses) typically completes in under 5 seconds. A `/16` (65K addresses) takes about a minute depending on your settings.

### Global Options

```bash
-v, --verbose                   Increase log verbosity (-v info, -vv debug, -vvv trace)
```

Logs are written to **stderr**, styled output to **stdout**. This means you can pipe ZeroTick's output without log noise getting in the way.

```bash
# See what's happening under the hood
zerotick -vv status mc.hypixel.net

# Full trace logging for debugging protocol issues
zerotick -vvv status some.broken.server.com
```

## How It Works 🧠

1. **Address resolution**: Parses the input, checks for explicit ports, queries DNS SRV records for `_minecraft._tcp.<domain>`
2. **TCP connect**: Opens an async connection with configurable timeout
3. **Handshake**: Sends a Minecraft protocol handshake packet (VarInt-encoded, length-prefixed)
4. **Status request**: Sends the status request packet, reads the JSON response
5. **Ping/pong**: Measures round-trip latency with a timestamp ping
6. **Mojang checks**: Fetches the blocked servers list over HTTPS, hashes the address with SHA-1
7. **Profile lookups**: For players in the sample list, fetches skin/cape URLs from the session server
8. **Rendering**: Parses Minecraft chat components, maps `§` codes and named colors to ANSI, outputs styled trees and tables

For scanning, steps 2-5 run concurrently across hundreds of async tasks, coordinated by a semaphore and rate limiter.

Everything uses raw TCP and the actual Minecraft wire protocol — no libraries abstracting it away. The HTTPS client is a minimal implementation on top of `rustls` that speaks just enough HTTP/1.1 to talk to Mojang's API.

## Contributing

PRs welcome! The codebase is organized into clean modules:

- `protocol/` — VarInt codec, packet framing, SLP implementations
- `net/` — Address resolution, HTTPS client
- `mojang/` — Blocked server checks, player profiles
- `scanner/` — CIDR parsing, async probe workers
- `style/` — MOTD rendering, tree/table output, color palette
- `commands/` — CLI command orchestration

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

Made with ☕ and raw TCP packets. If you've ever wondered "what's running on port 25565?", now you know.