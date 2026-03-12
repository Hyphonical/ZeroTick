
# ZeroTick ⚡

**Minecraft server reconnaissance from the command line.**

Ping servers, pull their status, see who's playing, check if they're blocked by Mojang —
all from raw protocol. No game client, no mods, no auth. Just TCP and curiosity.

## Table of Contents

- [What's This About?](#whats-this-about)
- [Why Does This Exist?](#why-does-this-exist)
- [Features](#features-)
- [Quick Start](#quick-start-)
- [Usage](#usage)
  - [`status` — Query a Server](#status--query-a-server)
  - [`scan` — Scan an IP Range](#scan--scan-an-ip-range)
  - [Global Options](#global-options)
- [How It Works](#how-it-works-)
- [Platform Support](#platform-support)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

## What's This About?

You ever wonder what's running on `play.someserver.net`? What version, how many
players, what the MOTD says, whether the server is on Mojang's blocklist?

Sure, you could open Minecraft, add it to your server list, squint at the tiny
tooltip, and hope the UI gives you everything. Or you could type:

```bash
zerotick status mc.hypixel.net
```

And get the full picture in your terminal, color-coded and formatted, in under a
second. Version, protocol, player list, latency, mod loader, secure chat status,
Mojang block status — everything the protocol exposes.

Oh, and if you're *really* curious, you can point it at an entire subnet:

```bash
zerotick scan 192.168.1.0/24
```

It'll probe every address for a Minecraft server and show you what it finds.
Politely. With rate limiting. We're not animals.

## Why Does This Exist?

Honestly? I wanted to know what was out there.

There are web-based Minecraft server status checkers, sure. They work fine for one
server at a time if you don't mind the ads and the five-second loading spinner for
what is fundamentally a single TCP handshake.

I wanted something **fast**, something I could script, something that didn't phone
home or require a browser. And I wanted to learn how the Minecraft protocol actually
works at the byte level — turns out it's a neat little VarInt-framed protocol that's
fun to implement from scratch.

So here's ZeroTick. Zero external APIs for the core functionality (the Mojang API
is optional, for blocklist checks and player skins). Zero bloat. Just you and the
wire.

Built in Rust because I like my tools compiled, single-binary, and unreasonably fast. ⚡

## Features 🎯

- **📡 Server List Ping** — full SLP handshake, modern protocol (1.7+) and legacy fallback
- **⏱️ Latency measurement** — real protocol-level ping/pong, not ICMP
- **👥 Player list** — online count, max, and the sample player list with UUIDs
- **📝 MOTD rendering** — Minecraft chat components rendered as ANSI-colored terminal text
- **🔒 Secure chat detection** — shows whether the server enforces signed messages
- **🧩 Mod detection** — identifies Forge/NeoForge/Fabric from the SLP response
- **🚫 Mojang blocklist check** — SHA-1 hash check against the official blocked servers list
- **🎨 Player profiles** — skin URLs and capes via the Mojang session API
- **🌐 DNS SRV resolution** — handles `_minecraft._tcp` records automatically
- **🔍 Range scanning** — probe entire CIDR ranges for Minecraft servers
- **⚡ Async I/O** — thousands of concurrent connections with configurable rate limiting
- **🎨 Styled output** — clean, color-coded terminal display with tree and table views
- **🖥️ Cross-platform** — Windows, macOS, Linux, every major terminal

## Quick Start 🚀

### Build from source

```bash
git clone https://github.com/yourname/zerotick.git
cd zerotick
cargo build --release
```

Binary lands at `target/release/zerotick` (or `zerotick.exe` on Windows).

### Query a server

```bash
zerotick status mc.hypixel.net
```

```text
● mc.hypixel.net:25565
 ├─ Version    1.8-1.21.4 (769)
 ├─ Latency    23ms
 ├─ Players    42,069/100,000
 │   ├─ Technoblade
 │   ├─ Dream
 │   ╰─ … 42,067 more
 ├─ MOTD       Hypixel Network [1.8-1.21]
 │              SUMMER EVENT
 ├─ Secure     enforced
 ╰─ Blocked    no
```

### Scan a range

```bash
zerotick scan 192.168.1.0/24
```

```text
● Scan 192.168.1.0/24:25565  (256 addresses)
  · Rate        1000/s
  · Timeout     3000ms

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  256/256

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

### `status` — Query a Server

```bash
zerotick status <ADDRESS> [OPTIONS]
```

| Flag | Default | Description |
|---|---|---|
| `<ADDRESS>` | *required* | Server address — `ip`, `ip:port`, `domain`, or `domain:port` |
| `-t, --timeout <MS>` | `5000` | Connection timeout in milliseconds |

Queries a single Minecraft server using the Server List Ping protocol. Automatically
resolves DNS SRV records for domain names. Falls back to the legacy ping protocol
for servers older than 1.7.

```bash
# Domain (SRV lookup + default port 25565)
zerotick status mc.hypixel.net

# Explicit IP and port
zerotick status 192.168.1.10:25565

# Shorter timeout for LAN servers
zerotick status 10.0.0.5 -t 1000
```

### `scan` — Scan an IP Range

```bash
zerotick scan <RANGE> [OPTIONS]
```

| Flag | Default | Description |
|---|---|---|
| `<RANGE>` | *required* | CIDR notation, e.g. `192.168.1.0/24` |
| `-p, --port <PORT>` | `25565` | Target port |
| `-c, --concurrency <N>` | `256` | Maximum concurrent connections |
| `-t, --timeout <MS>` | `3000` | Per-connection timeout |
| `--rate <N>` | `1000` | Maximum new connections per second |

Probes every address in the given CIDR range for a Minecraft server. Connections are
rate-limited and concurrent, so it's fast but not aggressive. Scan order is randomized
to avoid hammering sequential address blocks.

```bash
# Scan a /24 on the default port
zerotick scan 192.168.1.0/24

# Scan a /16 slowly and carefully
zerotick scan 10.0.0.0/16 --rate 200 -c 64 -t 2000

# Non-standard port
zerotick scan 172.16.0.0/24 -p 25566
```

> [!CAUTION]
> Scanning IP ranges you don't own may violate your ISP's terms of service or local laws.
> ZeroTick is a tool — what you point it at is your responsibility.

### Global Options

| Flag | Description |
|---|---|
| `-v` | Verbose logging (`info` level) |
| `-vv` | Debug logging |
| `-vvv` | Trace logging (very noisy) |
| `--version` | Print version |
| `--help` | Print help |

Logs go to **stderr**. Styled output goes to **stdout**. So you can do things like:

```bash
# Suppress logs, keep only the pretty output
zerotick status mc.hypixel.net 2>/dev/null

# Debug a connection issue
zerotick -vv status mc.hypixel.net
```

## How It Works 🧠

ZeroTick speaks the Minecraft protocol directly — no libraries, no wrappers. Here's
what happens when you run `zerotick status`:

1. **Resolve** — parse the address, check for a `_minecraft._tcp` DNS SRV record
2. **Connect** — open a TCP connection with a configurable timeout
3. **Handshake** — send a Minecraft handshake packet (VarInt-encoded, protocol version, target address, next-state = Status)
4. **Status Request** — send an empty `0x00` packet
5. **Status Response** — receive and parse the JSON blob (version, players, MOTD, favicon, mods, secure chat flag)
6. **Ping** — send a `0x01` packet with a timestamp, measure round-trip
7. **Mojang Check** *(optional)* — SHA-1 hash the server address, compare against Mojang's blocklist over HTTPS
8. **Render** — format everything into a styled tree and print to stdout

The entire exchange is ~4 packets. On a LAN server, the whole thing takes single-digit
milliseconds.

For legacy servers (pre-1.7), ZeroTick falls back to the `0xFE 0x01` legacy ping,
which returns a UTF-16BE string instead of JSON. Less data, but still gets you
version, MOTD, and player count.

## Platform Support

| OS | Terminal | Status |
|---|---|---|
| Linux | Alacritty, Kitty, GNOME Terminal, foot, any VTE-based | ✓ |
| macOS | Terminal.app, iTerm2, Alacritty, Kitty | ✓ |
| Windows | Windows Terminal, ConEmu, Alacritty | ✓ |
| Windows | Legacy `cmd.exe` / PowerShell < 7 | ⚠ colors may not render |

ZeroTick uses 24-bit TrueColor ANSI. If your terminal was released after 2016,
you're fine.

## Roadmap

- [x] Modern SLP (1.7+)
- [x] Legacy SLP (≤1.6)
- [x] DNS SRV resolution
- [x] MOTD → ANSI rendering
- [x] Mojang blocklist check
- [x] Player profile / skin lookup
- [x] CIDR range scanner
- [ ] UDP Query protocol (full player list, map name, plugins)
- [ ] Bedrock / MCPE ping (RakNet)
- [ ] RCON client
- [ ] Favicon → terminal pixel art
- [ ] JSON export
- [ ] Server software fingerprinting heuristics

## Contributing

PRs welcome. If you find a server that returns something ZeroTick doesn't parse
correctly, open an issue with the raw JSON and I'll fix it.

## License

MIT. See [LICENSE](LICENSE).

---

Built with Rust, TCP, and an unreasonable interest in Minecraft server metadata. ⚡