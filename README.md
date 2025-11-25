# cursedboard

Zero-config clipboard sync across devices via mDNS.

## Features

- **Zero-config**: devices discover each other automatically on the local network
- **Trust-first-seen**: new peers are automatically trusted on first connection
- **PSK authentication**: HMAC-SHA256 pre-shared key prevents unauthorized access
- **Cross-platform**: works on macOS and Linux

## Installation

### Nix

```bash
nix run github:uzaaft/cursedboard
```

Or add to your flake:

```nix
{
  inputs.cursedboard.url = "github:uzaaft/cursedboard";
}
```

### Cargo

```bash
cargo install --git https://github.com/uzaaft/cursedboard
```

## Usage

```bash
# Start with defaults (name: cursedboard, port: 42069, psk: cursedboard)
cursedboard

# Custom name and PSK
cursedboard --name mydevice --psk mysecret

# Or use environment variable for PSK
CURSEDBOARD_PSK=mysecret cursedboard
```

### Options

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `-n, --name` | | `cursedboard` | Device name for discovery |
| `-p, --port` | | `42069` | TCP port for connections |
| `--psk` | `CURSEDBOARD_PSK` | `cursedboard` | Pre-shared key for auth |
| `--poll-ms` | | `500` | Clipboard polling interval |

## How it works

1. On startup, registers mDNS service `_cursedboard._tcp.local.`
2. Browses for other cursedboard instances on the network
3. Connects to discovered peers and authenticates with PSK
4. Polls local clipboard for changes
5. Broadcasts clipboard changes to all connected peers
6. Receives clipboard changes from peers and applies them locally

## Security

- PSK authentication uses HMAC-SHA256 challenge-response
- Peers must share the same PSK to connect
- New peers are trusted on first successful connection
- Trusted peers are persisted in `~/.config/cursedboard/trusted.toml`

## License

MIT
