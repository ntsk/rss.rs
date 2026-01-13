mod cli;
mod config;
mod feed;
mod opml;
mod service;
mod subscription;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use subscription::SubscriptionManager;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config_path = config::get_config_path()?;
    debug!("Config path: {:?}", config_path);
    let mut manager = SubscriptionManager::new(&config_path)?;

    match cli.command {
        Some(Commands::Add { url }) => {
            info!("Adding feed: {}", url);
            print!("Fetching feed... ");
            let title = match feed::fetch_articles(&url) {
                Ok(articles) => {
                    println!("OK");
                    debug!("Fetched {} articles", articles.len());
                    articles.first().map(|a| a.feed_title.clone())
                }
                Err(e) => {
                    println!("Warning: {}", e);
                    warn!("Failed to fetch feed: {}", e);
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
            info!("Deleting feed: {}", url);
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
        Some(Commands::Config { key, value }) => {
            let mut settings = config::Settings::load()?;
            if let (Some(k), Some(v)) = (key, value) {
                let config_key: config::ConfigKey = k.parse()?;
                settings.set(config_key, &v)?;
                settings.save()?;
                println!("Set {} = {}", config_key, v);
            } else {
                println!("{}", settings.display());
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

    info!("Fetching {} feed(s)", feeds.len());
    let settings = config::Settings::load()?;

    let result = service::fetch_all_feeds(manager);

    for name in &result.failed_feeds {
        warn!("Failed to fetch: {}", name);
        eprintln!("Failed to fetch: {}", name);
    }

    debug!(
        "Fetched {} articles, {} failed",
        result.articles.len(),
        result.failed_feeds.len()
    );
    info!("Total {} articles", result.articles.len());

    if result.articles.is_empty() {
        println!("No articles found.");
        return Ok(());
    }

    ui::run_app(result.articles, &settings, result.feed_status)?;

    Ok(())
}
