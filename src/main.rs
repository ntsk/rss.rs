mod cli;
mod config;
mod feed;
mod subscription;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use subscription::SubscriptionManager;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config_path = config::get_config_path()?;
    let mut manager = SubscriptionManager::new(&config_path)?;

    match cli.command {
        Some(Commands::Add { url }) => {
            manager.add(&url)?;
            println!("Added: {}", url);
        }
        Some(Commands::Delete { url }) => {
            if manager.delete(&url)? {
                println!("Deleted: {}", url);
            } else {
                println!("Not found: {}", url);
            }
        }
        Some(Commands::List) => {
            let feeds = manager.list();
            if feeds.is_empty() {
                println!("No subscriptions yet. Use 'rss add <url>' to add a feed.");
            } else {
                for feed in feeds {
                    println!("{}", feed.url);
                }
            }
        }
        None => {
            show_articles(&manager)?;
        }
    }

    Ok(())
}


fn show_articles(manager: &SubscriptionManager) -> Result<()> {
    let feeds = manager.list();
    if feeds.is_empty() {
        println!("No subscriptions yet. Use 'rss add <url>' to add a feed.");
        return Ok(());
    }

    let mut articles: Vec<feed::Article> = Vec::new();
    for f in feeds {
        match feed::fetch_articles(&f.url) {
            Ok(mut fetched) => articles.append(&mut fetched),
            Err(e) => eprintln!("Failed to fetch {}: {}", f.url, e),
        }
    }

    if articles.is_empty() {
        println!("No articles found.");
        return Ok(());
    }

    articles.sort_by(|a, b| b.published.cmp(&a.published));

    ui::run_app(articles)?;

    Ok(())
}
