# rss.rs

[![test](https://github.com/ntsk/rss.rs/actions/workflows/test.yml/badge.svg)](https://github.com/ntsk/rss.rs/actions/workflows/test.yml)
[![lint](https://github.com/ntsk/rss.rs/actions/workflows/lint.yml/badge.svg)](https://github.com/ntsk/rss.rs/actions/workflows/lint.yml)

A terminal-based RSS/Atom feed reader with TUI.

```
┌Articles─────────────────────────────────────────────────────────────────────┐
│> 01/12 New Release: Version 2.0 is here                                     │
│        [Tech Blog]                                                          │
│  01/11 Understanding Rust Ownership                                         │
│        [Rust Weekly]                                                        │
│  01/10 10 Tips for Better Code Reviews                                      │
│        [Dev Community]                                                      │
│  01/09 Introduction to WebAssembly                                          │
│        [Mozilla Hacks]                                                      │
│                                                                             │
│                                                                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────────────────┐
│↑/↓: Navigate | Enter: Open | r: Reload | a: Add | l: List | q: Quit        │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Features

- RSS 2.0 and Atom feed support
- Terminal UI with keyboard navigation
- Auto-refresh (configurable interval)
- Manual refresh with `r` key
- Open articles in default browser
- OPML import/export
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

### Import feeds from OPML

```bash
rss import feeds.opml
```

### Export feeds to OPML

```bash
rss export feeds.opml
```

### Show settings

```bash
rss config --list
```

### Change settings

```bash
rss config auto_sort true
rss config refresh_interval_secs 600
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Enter` | Open article in browser |
| `r` | Reload feeds |
| `a` | Add new feed |
| `l` | Show feed list |
| `Ctrl+V` / `Cmd+V` | Paste URL (in add mode) |
| `q` / `Esc` | Quit |

### Feed List Mode

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `d` | Delete selected feed |
| `s` | Sort feeds |
| `Esc` | Back to articles |

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

# Auto-sort feeds when adding/deleting (default: false)
auto_sort = true
```

## Logging

Enable debug logging with the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug rss
RUST_LOG=rss=debug rss
```

## License

MIT
