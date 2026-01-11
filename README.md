# rss.rs

[![test](https://github.com/ntsk/rss.rs/actions/workflows/test.yml/badge.svg)](https://github.com/ntsk/rss.rs/actions/workflows/test.yml)
[![lint](https://github.com/ntsk/rss.rs/actions/workflows/lint.yml/badge.svg)](https://github.com/ntsk/rss.rs/actions/workflows/lint.yml)

A terminal-based RSS/Atom feed reader with TUI.

```
┌Articles───────────────────────────────────────────────────────────────┐
│> 01/12 New Release: Version 2.0 is here                               │
│        [Tech Blog]                                                    │
│  01/11 Understanding Rust Ownership                                   │
│        [Rust Weekly]                                                  │
│  01/10 10 Tips for Better Code Reviews                                │
│        [Dev Community]                                                │
│  01/09 Introduction to WebAssembly                                    │
│        [Mozilla Hacks]                                                │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
┌───────────────────────────────────────────────────────────────────────┐
│↑/↓: Navigate | Enter: View | o: Open | r: Reload | a: Add | l: List | q: Quit│
└───────────────────────────────────────────────────────────────────────┘
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
rss config
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
| `g` | Go to first item |
| `G` | Go to last item |
| `Ctrl+d` | Move down 10 items |
| `Ctrl+u` | Move up 10 items |
| `Enter` | View article content |
| `o` | Open article in browser |
| `r` | Reload feeds |
| `a` | Add new feed |
| `l` | Show feed list |
| `Ctrl+V` / `Cmd+V` / `p` | Paste URL (in add mode) |
| `q` / `Ctrl+c` | Quit |

### Article View Mode

| Key | Action |
|-----|--------|
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `g` | Scroll to top |
| `G` | Scroll to bottom |
| `Ctrl+d` | Scroll down half page |
| `Ctrl+u` | Scroll up half page |
| `Ctrl+f` | Scroll down full page |
| `Ctrl+b` | Scroll up full page |
| `o` | Open in browser |
| `h` / `q` / `Esc` | Back to list |

### Feed List Mode

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `g` | Go to first feed |
| `G` | Go to last feed |
| `Ctrl+d` | Move down 10 feeds |
| `Ctrl+u` | Move up 10 feeds |
| `Enter` | Open feed URL in browser |
| `a` | Add new feed |
| `d` | Delete selected feed |
| `s` | Sort feeds |
| `h` / `Esc` | Back to articles |

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
