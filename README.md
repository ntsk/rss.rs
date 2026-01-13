# rss.rs

[![test](https://github.com/ntsk/rss.rs/actions/workflows/test.yml/badge.svg)](https://github.com/ntsk/rss.rs/actions/workflows/test.yml)
[![lint](https://github.com/ntsk/rss.rs/actions/workflows/lint.yml/badge.svg)](https://github.com/ntsk/rss.rs/actions/workflows/lint.yml)

A fast, minimal RSS/Atom feed reader for the terminal.

## Features

- Vim-style keybindings
- RSS 2.0 and Atom support
- OPML import/export
- Article search (`/`, `n`, `N`)
- Auto-refresh

## Installation

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

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate |
| `Enter` | View article |
| `o` | Open in browser |
| `/` | Search |
| `n` / `N` | Next/prev match |
| `l` | Feed list |
| `a` | Add feed |
| `r` | Reload |
| `q` | Quit |

## Configuration

Config files are stored in `~/.config/rss/`.

```toml
# ~/.config/rss/config.toml
refresh_interval_secs = 300
auto_sort = false
```

## License

MIT
