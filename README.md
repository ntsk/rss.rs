# rss

A terminal-based RSS/Atom feed reader with TUI.

## Features

- RSS 2.0 and Atom feed support
- Terminal UI with keyboard navigation
- Auto-refresh every 5 minutes
- Manual refresh with `r` key
- Open articles in default browser
- XDG Base Directory compliant config storage

## Installation

### From source

```bash
cargo install --path .
```

### From crates.io

```bash
cargo install rss
```

## Usage

### View articles

```bash
rss
```

### Add a feed

```bash
rss add https://example.com/feed.xml
```

### Remove a feed

```bash
rss delete https://example.com/feed.xml
```

### List subscriptions

```bash
rss list
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Enter` | Open article in browser |
| `r` | Reload feeds |
| `q` / `Esc` | Quit |

## Configuration

Configuration files are stored in `~/.config/rss/` (or `$XDG_CONFIG_HOME/rss/`).

### Files

- `feeds.json` - Feed subscriptions
- `config.toml` - Application settings (optional)

### Settings

Create `~/.config/rss/config.toml` to customize:

```toml
# Auto-refresh interval in seconds (default: 300)
refresh_interval_secs = 600
```

## License

MIT
