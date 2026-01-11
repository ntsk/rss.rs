use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rss")]
#[command(about = "A CLI RSS reader")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
    Add {
        url: String,
    },
    Delete {
        url: String,
    },
    List,
    Import {
        file: PathBuf,
    },
    Export {
        file: PathBuf,
    },
    Config {
        #[arg(short, long)]
        list: bool,
        key: Option<String>,
        value: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_parse_add_command() {
        let cli = Cli::parse_from(["rss", "add", "https://example.com/feed.xml"]);

        match cli.command {
            Some(Commands::Add { url }) => {
                assert_eq!(url, "https://example.com/feed.xml");
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_parse_delete_command() {
        let cli = Cli::parse_from(["rss", "delete", "https://example.com/feed.xml"]);

        match cli.command {
            Some(Commands::Delete { url }) => {
                assert_eq!(url, "https://example.com/feed.xml");
            }
            _ => panic!("Expected Delete command"),
        }
    }

    #[test]
    fn test_parse_list_command() {
        let cli = Cli::parse_from(["rss", "list"]);

        match cli.command {
            Some(Commands::List) => {}
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_parse_no_command() {
        let cli = Cli::parse_from(["rss"]);

        assert!(cli.command.is_none());
    }

    #[test]
    fn test_parse_config_list() {
        let cli = Cli::parse_from(["rss", "config", "--list"]);

        match cli.command {
            Some(Commands::Config {
                list: true,
                key: None,
                value: None,
            }) => {}
            _ => panic!("Expected Config command with --list"),
        }
    }

    #[test]
    fn test_parse_config_set() {
        let cli = Cli::parse_from(["rss", "config", "auto_sort", "true"]);

        match cli.command {
            Some(Commands::Config {
                list: false,
                key: Some(k),
                value: Some(v),
            }) => {
                assert_eq!(k, "auto_sort");
                assert_eq!(v, "true");
            }
            _ => panic!("Expected Config command with key and value"),
        }
    }
}
