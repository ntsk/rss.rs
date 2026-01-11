use crate::config::Settings;
use crate::feed::{self, Article};
use crate::subscription::{Feed, SubscriptionManager};
use anyhow::Result;
use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use rayon::prelude::*;
use std::io::{self, stdout};
use std::time::{Duration, Instant};

const TICK_RATE_MS: u64 = 250;
const STATUS_MESSAGE_DURATION_SECS: u64 = 3;

pub fn run_app(articles: Vec<Article>, settings: &Settings) -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let refresh_interval = Duration::from_secs(settings.refresh_interval_secs);
    let mut app = App::new(articles, refresh_interval, settings.auto_sort);
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let result = run_event_loop(&mut terminal, &mut app, &mut list_state);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    list_state: &mut ListState,
) -> Result<()> {
    let tick_rate = Duration::from_millis(TICK_RATE_MS);

    loop {
        terminal.draw(|frame| draw_ui(frame, app, list_state))?;

        if app.should_quit {
            break;
        }

        app.clear_expired_status();

        if app.input_mode == InputMode::Normal && (app.should_auto_refresh() || app.should_reload) {
            let is_manual = app.should_reload;
            if is_manual {
                app.set_status("Reloading...");
                terminal.draw(|frame| draw_ui(frame, app, list_state))?;
            }
            if let Some(result) = fetch_all_articles() {
                let failure_msg = result.failure_message();
                if !result.articles.is_empty() {
                    app.update_articles(result.articles);
                }
                if let Some(msg) = failure_msg {
                    app.set_status(msg);
                } else if is_manual {
                    app.set_status("Reloaded");
                }
            }
        }

        if event::poll(tick_rate)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.select_next();
                        list_state.select(Some(app.selected));
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.select_previous();
                        list_state.select(Some(app.selected));
                    }
                    KeyCode::Enter => {
                        if let Some(article) = app.selected_article() {
                            let _ = open::that(&article.link);
                        }
                    }
                    KeyCode::Char('r') => app.request_reload(),
                    KeyCode::Char('a') => app.start_adding_feed(),
                    KeyCode::Char('l') => app.show_feed_list(),
                    _ => {}
                },
                InputMode::FeedList => match key.code {
                    KeyCode::Esc => app.close_feed_list(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next_feed(),
                    KeyCode::Up | KeyCode::Char('k') => app.select_previous_feed(),
                    KeyCode::Char('d') => delete_feed_and_refresh(app),
                    KeyCode::Char('s') => sort_feeds(app),
                    _ => {}
                },
                InputMode::AddingFeed => match key.code {
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Enter => {
                        if !app.input_buffer.is_empty() {
                            let url = app.input_buffer.clone();
                            app.input_mode = InputMode::Normal;
                            app.input_buffer.clear();
                            add_feed_and_refresh(app, &url);
                            list_state.select(Some(app.selected));
                        } else {
                            app.input_mode = InputMode::Normal;
                            app.input_buffer.clear();
                        }
                    }
                    KeyCode::Char('v')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            || key.modifiers.contains(KeyModifiers::SUPER) =>
                    {
                        if let Ok(mut clipboard) = Clipboard::new()
                            && let Ok(text) = clipboard.get_text()
                        {
                            app.input_buffer.push_str(&text);
                        }
                    }
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    _ => {}
                },
            }
        }
    }

    Ok(())
}

fn delete_feed_and_refresh(app: &mut App) {
    if app.feeds.is_empty() {
        return;
    }

    let feed = &app.feeds[app.feed_selected];
    let url = feed.url.clone();
    let name = feed.title.clone().unwrap_or_else(|| url.clone());

    let config_path = match crate::config::get_config_path() {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut manager = match SubscriptionManager::new(&config_path) {
        Ok(m) => m,
        Err(_) => return,
    };

    if manager.delete(&url).is_ok() {
        if app.auto_sort {
            let _ = manager.sort();
        }
        app.feeds = manager.list().to_vec();
        if !app.feeds.is_empty() && app.feed_selected >= app.feeds.len() {
            app.feed_selected = app.feeds.len() - 1;
        }
        if let Some(result) = fetch_all_articles()
            && !result.articles.is_empty()
        {
            app.update_articles(result.articles);
        }
        app.set_status(format!("Deleted: {}", name));
    }
}

fn sort_feeds(app: &mut App) {
    let config_path = match crate::config::get_config_path() {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut manager = match SubscriptionManager::new(&config_path) {
        Ok(m) => m,
        Err(_) => return,
    };

    if manager.sort().is_ok() {
        app.feeds = manager.list().to_vec();
        app.feed_selected = 0;
        app.set_status("Sorted");
    }
}

fn add_feed_and_refresh(app: &mut App, url: &str) {
    let config_path = match crate::config::get_config_path() {
        Ok(p) => p,
        Err(_) => {
            app.set_status("Failed to get config path");
            return;
        }
    };
    let mut manager = match SubscriptionManager::new(&config_path) {
        Ok(m) => m,
        Err(_) => {
            app.set_status("Failed to load subscriptions");
            return;
        }
    };

    let title = feed::fetch_articles(url)
        .ok()
        .and_then(|articles| articles.first().map(|a| a.feed_title.clone()));

    match manager.add(url, title.clone()) {
        Ok(_) => {
            if app.auto_sort {
                let _ = manager.sort();
            }
            if let Some(result) = fetch_all_articles()
                && !result.articles.is_empty()
            {
                app.update_articles(result.articles);
            }
            let name = title.unwrap_or_else(|| url.to_string());
            app.set_status(format!("Added: {}", name));
        }
        Err(e) => {
            app.set_status(format!("Error: {}", e));
        }
    }
}

fn draw_ui(frame: &mut Frame, app: &App, list_state: &mut ListState) {
    if app.input_mode == InputMode::FeedList {
        draw_feed_list(frame, app);
    } else {
        draw_article_list(frame, app, list_state);
    }
}

fn draw_feed_list(frame: &mut Frame, app: &App) {
    let bottom_height = if app.status_message.is_some() { 6 } else { 3 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(bottom_height)])
        .split(frame.area());

    let items: Vec<ListItem> = app
        .feeds
        .iter()
        .map(|f| {
            let display = match &f.title {
                Some(title) => format!("{} ({})", title, f.url),
                None => f.url.clone(),
            };
            ListItem::new(display)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Feeds"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    let mut feed_list_state = ListState::default();
    feed_list_state.select(Some(app.feed_selected));
    frame.render_stateful_widget(list, chunks[0], &mut feed_list_state);

    let bottom_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if app.status_message.is_some() {
            vec![Constraint::Length(3), Constraint::Length(3)]
        } else {
            vec![Constraint::Length(3)]
        })
        .split(chunks[1]);

    if let Some(msg) = &app.status_message {
        let status = Paragraph::new(msg.as_str())
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, bottom_chunks[0]);

        let help = Paragraph::new("↑/↓: Navigate | d: Delete | s: Sort | Esc: Back")
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[1]);
    } else {
        let help = Paragraph::new("↑/↓: Navigate | d: Delete | s: Sort | Esc: Back")
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[0]);
    }
}

fn draw_article_list(frame: &mut Frame, app: &App, list_state: &mut ListState) {
    let bottom_height = if app.input_mode == InputMode::AddingFeed || app.status_message.is_some() {
        6
    } else {
        3
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(bottom_height)])
        .split(frame.area());

    let items: Vec<ListItem> = app
        .articles
        .iter()
        .map(|a| {
            let date = a
                .published
                .map(|d| d.format("%m/%d").to_string())
                .unwrap_or_else(|| "     ".to_string());
            let line1 = Line::from(format!("{} {}", date, a.title));
            let line2 = Line::from(vec![Span::styled(
                format!("      [{}]", a.feed_title),
                Style::default().fg(Color::Gray),
            )]);
            ListItem::new(Text::from(vec![line1, line2]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Articles"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, chunks[0], list_state);

    let bottom_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            if app.input_mode == InputMode::AddingFeed || app.status_message.is_some() {
                vec![Constraint::Length(3), Constraint::Length(3)]
            } else {
                vec![Constraint::Length(3)]
            },
        )
        .split(chunks[1]);

    if app.input_mode == InputMode::AddingFeed {
        let input = Paragraph::new(app.input_buffer.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Add Feed URL (Enter to add, Esc to cancel)"),
        );
        frame.render_widget(input, bottom_chunks[0]);

        let help = Paragraph::new("Type feed URL and press Enter")
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[1]);
    } else if let Some(msg) = &app.status_message {
        let status = Paragraph::new(msg.as_str())
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, bottom_chunks[0]);

        let help =
            Paragraph::new("↑/↓: Navigate | Enter: Open | r: Reload | a: Add | l: List | q: Quit")
                .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[1]);
    } else {
        let help =
            Paragraph::new("↑/↓: Navigate | Enter: Open | r: Reload | a: Add | l: List | q: Quit")
                .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[0]);
    }
}

struct FetchResult {
    articles: Vec<Article>,
    failed_feeds: Vec<String>,
}

impl FetchResult {
    fn failure_message(&self) -> Option<String> {
        if self.failed_feeds.is_empty() {
            return None;
        }

        if self.failed_feeds.len() == 1 {
            Some(format!("Failed: {}", self.failed_feeds[0]))
        } else {
            Some(format!("Failed: {} feeds", self.failed_feeds.len()))
        }
    }
}

fn fetch_all_articles() -> Option<FetchResult> {
    let config_path = crate::config::get_config_path().ok()?;
    let manager = SubscriptionManager::new(&config_path).ok()?;
    let feeds = manager.list();

    let results: Vec<_> = feeds
        .par_iter()
        .map(|f| {
            let name = f.title.clone().unwrap_or_else(|| f.url.clone());
            match feed::fetch_articles(&f.url) {
                Ok(articles) => (articles, None),
                Err(_) => (vec![], Some(name)),
            }
        })
        .collect();

    let mut articles: Vec<Article> = results.iter().flat_map(|(a, _)| a.clone()).collect();
    let failed_feeds: Vec<String> = results.iter().filter_map(|(_, f)| f.clone()).collect();

    if articles.is_empty() && failed_feeds.is_empty() {
        return None;
    }

    articles.sort_by(|a, b| b.published.cmp(&a.published));
    Some(FetchResult {
        articles,
        failed_feeds,
    })
}

#[derive(Debug, Default, PartialEq)]
pub enum InputMode {
    #[default]
    Normal,
    AddingFeed,
    FeedList,
}

pub struct App {
    pub articles: Vec<Article>,
    pub selected: usize,
    pub should_quit: bool,
    pub should_reload: bool,
    pub last_refresh: Instant,
    pub refresh_interval: Duration,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub status_message: Option<String>,
    pub status_message_time: Option<Instant>,
    pub feeds: Vec<Feed>,
    pub feed_selected: usize,
    pub auto_sort: bool,
}

impl App {
    pub fn new(articles: Vec<Article>, refresh_interval: Duration, auto_sort: bool) -> Self {
        Self {
            articles,
            selected: 0,
            should_quit: false,
            should_reload: false,
            last_refresh: Instant::now(),
            refresh_interval,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            status_message: None,
            status_message_time: None,
            feeds: Vec::new(),
            feed_selected: 0,
            auto_sort,
        }
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
        self.status_message_time = Some(Instant::now());
    }

    pub fn clear_expired_status(&mut self) {
        if let Some(time) = self.status_message_time
            && time.elapsed() >= Duration::from_secs(STATUS_MESSAGE_DURATION_SECS)
        {
            self.clear_status();
        }
    }

    pub fn select_next(&mut self) {
        if !self.articles.is_empty() && self.selected < self.articles.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn request_reload(&mut self) {
        self.should_reload = true;
    }

    pub fn should_auto_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= self.refresh_interval
    }

    pub fn update_articles(&mut self, articles: Vec<Article>) {
        self.articles = articles;
        if !self.articles.is_empty() && self.selected >= self.articles.len() {
            self.selected = self.articles.len() - 1;
        }
        self.last_refresh = Instant::now();
        self.should_reload = false;
    }

    pub fn selected_article(&self) -> Option<&Article> {
        self.articles.get(self.selected)
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
        self.status_message_time = None;
    }

    pub fn start_adding_feed(&mut self) {
        self.input_mode = InputMode::AddingFeed;
        self.input_buffer.clear();
        self.clear_status();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
    }

    pub fn show_feed_list(&mut self) {
        if let Ok(config_path) = crate::config::get_config_path()
            && let Ok(manager) = SubscriptionManager::new(&config_path)
        {
            self.feeds = manager.list().to_vec();
            self.feed_selected = 0;
            self.input_mode = InputMode::FeedList;
        }
    }

    pub fn select_next_feed(&mut self) {
        if !self.feeds.is_empty() && self.feed_selected < self.feeds.len() - 1 {
            self.feed_selected += 1;
        }
    }

    pub fn select_previous_feed(&mut self) {
        if self.feed_selected > 0 {
            self.feed_selected -= 1;
        }
    }

    pub fn close_feed_list(&mut self) {
        self.input_mode = InputMode::Normal;
        self.feeds.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_articles(count: usize) -> Vec<Article> {
        (0..count)
            .map(|i| Article {
                title: format!("Article {}", i),
                link: format!("https://example.com/{}", i),
                published: Some(Utc::now()),
                feed_title: "Test Feed".to_string(),
            })
            .collect()
    }

    #[test]
    fn test_new_app_with_articles() {
        let articles = create_test_articles(3);
        let app = App::new(articles.clone(), Duration::from_secs(300), false);

        assert_eq!(app.articles.len(), 3);
        assert_eq!(app.selected, 0);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_select_next() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);

        app.select_next();
        assert_eq!(app.selected, 1);

        app.select_next();
        assert_eq!(app.selected, 2);

        app.select_next();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_select_previous() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.selected = 2;

        app.select_previous();
        assert_eq!(app.selected, 1);

        app.select_previous();
        assert_eq!(app.selected, 0);

        app.select_previous();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_quit() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);

        app.quit();

        assert!(app.should_quit);
    }

    #[test]
    fn test_request_reload() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);

        app.request_reload();

        assert!(app.should_reload);
    }

    #[test]
    fn test_should_auto_refresh_after_interval() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(0), false);
        app.last_refresh = Instant::now() - Duration::from_secs(1);

        assert!(app.should_auto_refresh());
    }

    #[test]
    fn test_should_not_auto_refresh_before_interval() {
        let articles = create_test_articles(3);
        let app = App::new(articles, Duration::from_secs(300), false);

        assert!(!app.should_auto_refresh());
    }

    #[test]
    fn test_update_articles_preserves_selection_if_valid() {
        let articles = create_test_articles(5);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.selected = 2;

        let new_articles = create_test_articles(5);
        app.update_articles(new_articles);

        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_update_articles_adjusts_selection_if_out_of_bounds() {
        let articles = create_test_articles(5);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.selected = 4;

        let new_articles = create_test_articles(2);
        app.update_articles(new_articles);

        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_selected_article() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.selected = 1;

        let article = app.selected_article().unwrap();
        assert_eq!(article.title, "Article 1");
    }

    #[test]
    fn test_selected_article_empty_list() {
        let app = App::new(vec![], Duration::from_secs(300), false);

        assert!(app.selected_article().is_none());
    }

    #[test]
    fn test_start_adding_feed() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.input_buffer = "old".to_string();
        app.status_message = Some("old message".to_string());

        app.start_adding_feed();

        assert_eq!(app.input_mode, InputMode::AddingFeed);
        assert!(app.input_buffer.is_empty());
        assert!(app.status_message.is_none());
    }

    #[test]
    fn test_cancel_input() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.input_mode = InputMode::AddingFeed;
        app.input_buffer = "https://example.com".to_string();

        app.cancel_input();

        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.input_buffer.is_empty());
    }

    #[test]
    fn test_select_next_feed() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.feeds = vec![
            Feed {
                url: "https://example.com/1".to_string(),
                title: None,
            },
            Feed {
                url: "https://example.com/2".to_string(),
                title: None,
            },
        ];

        app.select_next_feed();
        assert_eq!(app.feed_selected, 1);

        app.select_next_feed();
        assert_eq!(app.feed_selected, 1);
    }

    #[test]
    fn test_select_previous_feed() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.feeds = vec![
            Feed {
                url: "https://example.com/1".to_string(),
                title: None,
            },
            Feed {
                url: "https://example.com/2".to_string(),
                title: None,
            },
        ];
        app.feed_selected = 1;

        app.select_previous_feed();
        assert_eq!(app.feed_selected, 0);

        app.select_previous_feed();
        assert_eq!(app.feed_selected, 0);
    }

    #[test]
    fn test_close_feed_list() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.input_mode = InputMode::FeedList;
        app.feeds = vec![Feed {
            url: "https://example.com".to_string(),
            title: None,
        }];

        app.close_feed_list();

        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.feeds.is_empty());
    }

    #[test]
    fn test_set_status() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);

        app.set_status("Test message");

        assert_eq!(app.status_message, Some("Test message".to_string()));
        assert!(app.status_message_time.is_some());
    }

    #[test]
    fn test_clear_status() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300), false);
        app.set_status("Test message");

        app.clear_status();

        assert!(app.status_message.is_none());
        assert!(app.status_message_time.is_none());
    }

    #[test]
    fn test_fetch_result_no_failures() {
        let result = FetchResult {
            articles: create_test_articles(3),
            failed_feeds: vec![],
        };

        assert!(result.failure_message().is_none());
    }

    #[test]
    fn test_fetch_result_single_failure() {
        let result = FetchResult {
            articles: vec![],
            failed_feeds: vec!["https://example.com/feed.xml".to_string()],
        };

        let message = result.failure_message().unwrap();
        assert!(message.contains("Failed"));
        assert!(message.contains("https://example.com/feed.xml"));
    }

    #[test]
    fn test_fetch_result_multiple_failures() {
        let result = FetchResult {
            articles: vec![],
            failed_feeds: vec![
                "https://example.com/feed1.xml".to_string(),
                "https://example.com/feed2.xml".to_string(),
            ],
        };

        let message = result.failure_message().unwrap();
        assert!(message.contains("Failed"));
        assert!(message.contains("2"));
    }
}
