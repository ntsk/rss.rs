mod cli;
mod config;
mod feed;
mod opml;
mod subscription;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use rayon::prelude::*;
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
            print!("Fetching feed... ");
            let title = match feed::fetch_articles(&url) {
                Ok(articles) => {
                    println!("OK");
                    articles.first().map(|a| a.feed_title.clone())
                }
                Err(e) => {
                    println!("Warning: {}", e);
                    None
                }
            };
            manager.add(&url, title.clone())?;
            match title {
                Some(t) => println!("Added: {} ({})", t, url),
                None => println!("Added: {}", url),
            }
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
                    match &feed.title {
                        Some(title) => println!("{} ({})", title, feed.url),
                        None => println!("{}", feed.url),
                    }
                }
            }
        }
        Some(Commands::Import { file }) => {
            let feeds = opml::import(&file)?;
            let mut imported = 0;
            for feed in feeds {
                if manager.add(&feed.url, feed.title).is_ok() {
                    imported += 1;
                }
            }
            println!("Imported {} feed(s)", imported);
        }
        Some(Commands::Export { file }) => {
            let feeds = manager.list();
            opml::export(&file, feeds)?;
            println!("Exported {} feed(s) to {:?}", feeds.len(), file);
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

    let settings = config::Settings::load()?;

    let results: Vec<_> = feeds
        .par_iter()
        .map(|f| (f.url.clone(), feed::fetch_articles(&f.url)))
        .collect();

    let mut articles: Vec<feed::Article> = Vec::new();
    for (url, result) in results {
        match result {
            Ok(mut fetched) => articles.append(&mut fetched),
            Err(e) => eprintln!("Failed to fetch {}: {}", url, e),
        }
    }

    if articles.is_empty() {
        println!("No articles found.");
        return Ok(());
    }

    articles.sort_by(|a, b| b.published.cmp(&a.published));

    ui::run_app(articles, &settings)?;

    Ok(())
}
