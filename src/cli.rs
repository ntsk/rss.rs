use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rss")]
#[command(about = "A CLI RSS reader")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Add {
        url: String,
    },
    Delete {
        url: String,
    },
    List,
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
}
