# Cursedboard

Zero-config clipboard synchronization across macOS and Linux machines.

## Quick Start

Simply run `cursedboard` on two machines on the same network:

```bash
# On Machine A (macOS or Linux)
cursedboard

# On Machine B (macOS or Linux)  
cursedboard
```

They'll automatically discover each other via mDNS and start syncing clipboards! 🎉

## How It Works

- **Zero Configuration**: No IP addresses, no config files needed
- **mDNS Discovery**: Uses Bonjour (macOS) / Avahi (Linux) for automatic peer discovery
- **First-Seen Trust**: First peer you connect to is automatically trusted
- **Group Scoping**: Only connects to peers with matching group (username by default)
- **Symmetric P2P**: Every instance is both client and server

## Advanced Usage

### Pairing Mode

Accept any new peer for 60 seconds:

```bash
cursedboard --pair 60
```

### Custom Group

Sync only with specific devices:

```bash
cursedboard --group my-team
```

### Pre-Shared Key

Add simple authentication:

```bash
cursedboard --psk my-secret-passphrase
```

### Manual Mode (No Discovery)

Disable auto-discovery:

```bash
cursedboard --no-discovery
```

### All Options

```bash
cursedboard --help
```

## Technical Details

- **Port**: 34254 (TCP)
- **Protocol**: Length-prefixed binary messages
- **Discovery**: mDNS service `_cursedboard._tcp.local.`
- **Security**: First-seen trust + optional PSK authentication
- **State**: Persisted in `~/.config/cursedboard/instance.toml`

## Architecture

Each instance runs:
1. **TCP Listener**: Accept incoming connections (port 34254)
2. **mDNS Service**: Advertise and discover peers  
3. **Clipboard Monitor**: Watch for local clipboard changes
4. **Connection Manager**: Handle peer connections with deduplication

## Building

```bash
cargo build --release
```

## Configuration File (Optional)

While cursedboard works without config, you can optionally create `~/.config/cursedboard/config.toml` for advanced settings. See `examples/` directory for samples.
