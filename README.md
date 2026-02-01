# rss.rs

[![test](https://github.com/ntsk/rss.rs/actions/workflows/test.yml/badge.svg)](https://github.com/ntsk/rss.rs/actions/workflows/test.yml)
[![lint](https://github.com/ntsk/rss.rs/actions/workflows/lint.yml/badge.svg)](https://github.com/ntsk/rss.rs/actions/workflows/lint.yml)

A simple RSS/Atom feed reader for the terminal.

## Features

- Vim-style keybindings
- RSS 1.0/2.0 and Atom support
- OPML import/export
- Reader mode with readability extraction
- Auto-refresh

## Installation

### Homebrew

```bash
brew install ntsk/tap/rss
```

### Nix

```bash
nix run github:ntsk/rss.rs
```

### From source

```bash
cargo install --path .
```

## Usage

```bash
rss                              # View articles
rss add <url>                    # Add feed
rss delete <url>                 # Remove feed
rss list                         # List feeds
rss import <file.opml>           # Import from OPML
rss export <file.opml>           # Export to OPML
```

## Keybindings

### Article List

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `g` / `G` | Go to top/bottom |
| `Ctrl+d` / `Ctrl+u` | Half page down/up |
| `Enter` | View article |
| `o` | Open in browser |
| `/` | Search |
| `n` / `N` | Next/prev match |
| `l` | Feed list |
| `a` | Add feed |
| `r` | Reload |
| `q` | Quit |

### Article View

| Key | Action |
|-----|--------|
| `j` / `k` | Move cursor down/up |
| `h` / `l` | Move cursor left/right |
| `g` / `G` | Go to top/bottom |
| `0` / `$` | Go to line start/end |
| `Ctrl+d` / `Ctrl+u` | Half page down/up |
| `Ctrl+f` / `Ctrl+b` | Full page down/up |
| `v` | Visual select (character) |
| `V` | Visual select (line) |
| `y` | Yank (copy) selection |
| `o` | Open in browser |
| `q` / `Esc` | Back to list |

### Feed List

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `g` / `G` | Go to top/bottom |
| `Enter` | Filter by feed |
| `o` | Open feed URL |
| `a` | Add feed |
| `d` | Delete feed |
| `s` | Sort feeds |
| `Esc` | Back to list |

## Configuration

Config files are stored in `~/.config/rss/`.

```toml
# ~/.config/rss/config.toml
refresh_interval_secs = 300
auto_sort = false
```

## License

MIT
