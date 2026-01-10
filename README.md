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

Feeds are stored in `~/.config/rss/feeds.json` (or `$XDG_CONFIG_HOME/rss/feeds.json`).

## License

MIT
