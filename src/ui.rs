use crate::config::Settings;
use crate::feed::{self, Article};
use crate::service::{self, FetchResult};
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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::collections::HashMap;
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
                app.apply_feed_status(result.feed_status);
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
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.quit()
                    }
                    KeyCode::Char('q') => app.quit(),
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.select_next();
                        list_state.select(Some(app.selected));
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.select_previous();
                        list_state.select(Some(app.selected));
                    }
                    KeyCode::Char('g') => {
                        app.select_first();
                        list_state.select(Some(app.selected));
                    }
                    KeyCode::Char('G') => {
                        app.select_last();
                        list_state.select(Some(app.selected));
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        for _ in 0..10 {
                            app.select_next();
                        }
                        list_state.select(Some(app.selected));
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        for _ in 0..10 {
                            app.select_previous();
                        }
                        list_state.select(Some(app.selected));
                    }
                    KeyCode::Enter => {
                        if let Some(article) = app.selected_article() {
                            let link = article.link.clone();
                            let width = terminal.size()?.width.saturating_sub(4) as usize;
                            app.set_status("Loading...");
                            terminal.draw(|frame| draw_ui(frame, app, list_state))?;
                            match feed::fetch_article_content(&link, width.max(40)) {
                                Ok(content) => {
                                    app.article_content = Some(content);
                                    app.article_scroll = 0;
                                    app.input_mode = InputMode::ViewingArticle;
                                    app.clear_status();
                                }
                                Err(e) => {
                                    app.set_status(format!("Failed to load: {}", e));
                                }
                            }
                        }
                    }
                    KeyCode::Char('o') => {
                        if let Some(article) = app.selected_article()
                            && let Err(e) = open::that(&article.link)
                        {
                            app.set_status(format!("Failed to open browser: {}", e));
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
                    KeyCode::Char('g') => app.select_first_feed(),
                    KeyCode::Char('G') => app.select_last_feed(),
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        for _ in 0..10 {
                            app.select_previous_feed();
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(feed) = app.feeds.get(app.feed_selected)
                            && let Err(e) = open::that(&feed.url)
                        {
                            app.set_status(format!("Failed to open browser: {}", e));
                        }
                    }
                    KeyCode::Char('a') => app.start_adding_feed(),
                    KeyCode::Char('d') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        delete_feed_and_refresh(app)
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        for _ in 0..10 {
                            app.select_next_feed();
                        }
                    }
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
                        paste_from_clipboard(app);
                    }
                    KeyCode::Char('p') => {
                        paste_from_clipboard(app);
                    }
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    _ => {}
                },
                InputMode::ViewingArticle => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.input_mode = InputMode::Normal;
                        app.article_content = None;
                        app.article_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.article_scroll = app.article_scroll.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.article_scroll = app.article_scroll.saturating_sub(1);
                    }
                    KeyCode::Char('g') => {
                        app.article_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        app.article_scroll = usize::MAX;
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.article_scroll = app.article_scroll.saturating_add(15);
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.article_scroll = app.article_scroll.saturating_sub(15);
                    }
                    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.article_scroll = app.article_scroll.saturating_add(30);
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.article_scroll = app.article_scroll.saturating_sub(30);
                    }
                    KeyCode::Char('o') => {
                        if let Some(article) = app.selected_article()
                            && let Err(e) = open::that(&article.link)
                        {
                            app.set_status(format!("Failed to open browser: {}", e));
                        }
                    }
                    _ => {}
                },
            }
        }
    }

    Ok(())
}

fn paste_from_clipboard(app: &mut App) {
    if let Ok(mut clipboard) = Clipboard::new()
        && let Ok(text) = clipboard.get_text()
    {
        app.input_buffer.push_str(&text);
    }
}

fn get_manager(app: &mut App) -> Option<SubscriptionManager> {
    let config_path = match crate::config::get_config_path() {
        Ok(p) => p,
        Err(e) => {
            app.set_status(format!("Config error: {}", e));
            return None;
        }
    };
    match SubscriptionManager::new(&config_path) {
        Ok(m) => Some(m),
        Err(e) => {
            app.set_status(format!("Load error: {}", e));
            None
        }
    }
}

fn refresh_after_change(app: &mut App, manager: &SubscriptionManager) {
    app.feeds = manager.list().to_vec();
    if let Some(result) = fetch_all_articles() {
        app.apply_feed_status(result.feed_status);
        if !result.articles.is_empty() {
            app.update_articles(result.articles);
        }
    }
}

fn delete_feed_and_refresh(app: &mut App) {
    if app.feeds.is_empty() {
        return;
    }

    let feed = &app.feeds[app.feed_selected];
    let url = feed.url.clone();
    let name = feed.title.clone().unwrap_or_else(|| url.clone());

    let Some(mut manager) = get_manager(app) else {
        return;
    };

    if manager.delete(&url).is_ok() {
        if app.auto_sort
            && let Err(e) = manager.sort()
        {
            app.set_status(format!("Sort failed: {}", e));
        }
        refresh_after_change(app, &manager);
        if !app.feeds.is_empty() && app.feed_selected >= app.feeds.len() {
            app.feed_selected = app.feeds.len() - 1;
        }
        app.set_status(format!("Deleted: {}", name));
    }
}

fn sort_feeds(app: &mut App) {
    let Some(mut manager) = get_manager(app) else {
        return;
    };

    match manager.sort() {
        Ok(_) => {
            app.feeds = manager.list().to_vec();
            app.feed_selected = 0;
            app.set_status("Sorted");
        }
        Err(e) => {
            app.set_status(format!("Sort failed: {}", e));
        }
    }
}

fn add_feed_and_refresh(app: &mut App, url: &str) {
    let Some(mut manager) = get_manager(app) else {
        return;
    };

    let title = feed::fetch_articles(url)
        .ok()
        .and_then(|articles| articles.first().map(|a| a.feed_title.clone()));

    match manager.add(url, title.clone()) {
        Ok(_) => {
            if app.auto_sort
                && let Err(e) = manager.sort()
            {
                app.set_status(format!("Sort failed: {}", e));
            }
            refresh_after_change(app, &manager);
            let name = title.unwrap_or_else(|| url.to_string());
            app.set_status(format!("Added: {}", name));
        }
        Err(e) => {
            app.set_status(format!("Error: {}", e));
        }
    }
}

fn wrap_text(s: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![s.to_string()];
    }
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for c in s.chars() {
        if current_width >= max_width {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
        }
        current_line.push(c);
        current_width += 1;
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn truncate_str(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else if max_width <= 3 {
        s.chars().take(max_width).collect()
    } else {
        let truncated: String = s.chars().take(max_width - 3).collect();
        format!("{}...", truncated)
    }
}

fn draw_ui(frame: &mut Frame, app: &App, list_state: &mut ListState) {
    match app.input_mode {
        InputMode::FeedList => draw_feed_list(frame, app),
        InputMode::ViewingArticle => draw_article_content(frame, app),
        _ => draw_article_list(frame, app, list_state),
    }
}

fn draw_feed_list(frame: &mut Frame, app: &App) {
    let help_text = "↑/↓: Navigate | Enter: Open | a: Add | d: Delete | s: Sort | Esc: Back";
    let available_width = frame.area().width.saturating_sub(4) as usize;
    let help_lines = if available_width > 0 {
        help_text.chars().count().div_ceil(available_width)
    } else {
        1
    };
    let help_height = (help_lines as u16) + 2;

    let bottom_height = if app.status_message.is_some() {
        help_height + 3
    } else {
        help_height
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(bottom_height)])
        .split(frame.area());

    let available_width = chunks[0].width.saturating_sub(6) as usize;
    let items: Vec<ListItem> = app
        .feeds
        .iter()
        .map(|f| {
            let status_icon = match app.get_feed_status(&f.url) {
                Some(true) => Span::styled("✓ ", Style::default().fg(Color::Green)),
                Some(false) => Span::styled("✗ ", Style::default().fg(Color::Red)),
                None => Span::raw("  "),
            };
            let display = match &f.title {
                Some(title) => format!("{} ({})", title, f.url),
                None => f.url.clone(),
            };
            let display = truncate_str(&display, available_width);
            ListItem::new(Line::from(vec![status_icon, Span::raw(display)]))
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
            vec![Constraint::Length(3), Constraint::Length(help_height)]
        } else {
            vec![Constraint::Length(help_height)]
        })
        .split(chunks[1]);

    if let Some(msg) = &app.status_message {
        let status = Paragraph::new(msg.as_str())
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, bottom_chunks[0]);

        let help = Paragraph::new(help_text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[1]);
    } else {
        let help = Paragraph::new(help_text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[0]);
    }
}

fn draw_article_content(frame: &mut Frame, app: &App) {
    let help_text = "↑/↓: Scroll | o: Open in browser | q/Esc: Back";
    let available_width = frame.area().width.saturating_sub(4) as usize;
    let help_lines = if available_width > 0 {
        help_text.chars().count().div_ceil(available_width)
    } else {
        1
    };
    let help_height = (help_lines as u16) + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(help_height)])
        .split(frame.area());

    let title = app
        .selected_article()
        .map(|a| a.title.clone())
        .unwrap_or_default();

    let content = app.article_content.as_deref().unwrap_or("");
    let lines: Vec<&str> = content.lines().collect();
    let visible_height = chunks[0].height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(visible_height);
    let scroll = app.article_scroll.min(max_scroll);

    let visible_lines: String = lines
        .iter()
        .skip(scroll)
        .take(visible_height)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");

    let content_widget =
        Paragraph::new(visible_lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(content_widget, chunks[0]);

    let help = Paragraph::new(help_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[1]);
}

fn draw_article_list(frame: &mut Frame, app: &App, list_state: &mut ListState) {
    let help_text = "↑/↓: Navigate | Enter: View | o: Open | r: Reload | a: Add | l: List | q: Quit";
    let available_width = frame.area().width.saturating_sub(4) as usize;
    let help_lines = if available_width > 0 {
        help_text.chars().count().div_ceil(available_width)
    } else {
        1
    };
    let help_height = (help_lines as u16) + 2;

    let bottom_height = if app.input_mode == InputMode::AddingFeed || app.status_message.is_some() {
        help_height + 3
    } else {
        help_height
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(bottom_height)])
        .split(frame.area());

    let available_width = chunks[0].width.saturating_sub(4) as usize;
    let items: Vec<ListItem> = app
        .articles
        .iter()
        .map(|a| {
            let date = a
                .published
                .map(|d| d.format("%m/%d").to_string())
                .unwrap_or_else(|| "     ".to_string());
            let title_max = available_width.saturating_sub(7);
            let title_lines = wrap_text(&a.title, title_max);
            let mut lines: Vec<Line> = title_lines
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    if i == 0 {
                        Line::from(format!("{} {}", date, line))
                    } else {
                        Line::from(format!("      {}", line))
                    }
                })
                .collect();
            let feed_max = available_width.saturating_sub(8);
            let feed_title = truncate_str(&a.feed_title, feed_max);
            lines.push(Line::from(vec![Span::styled(
                format!("      [{}]", feed_title),
                Style::default().fg(Color::Gray),
            )]));
            ListItem::new(Text::from(lines))
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
                vec![Constraint::Length(3), Constraint::Length(help_height)]
            } else {
                vec![Constraint::Length(help_height)]
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
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[1]);
    } else if let Some(msg) = &app.status_message {
        let status = Paragraph::new(msg.as_str())
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, bottom_chunks[0]);

        let help = Paragraph::new(help_text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[1]);
    } else {
        let help = Paragraph::new(help_text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, bottom_chunks[0]);
    }
}

fn fetch_all_articles() -> Option<FetchResult> {
    service::fetch_feeds_from_config()
}

#[derive(Debug, Default, PartialEq)]
pub enum InputMode {
    #[default]
    Normal,
    AddingFeed,
    FeedList,
    ViewingArticle,
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
    pub feed_status: HashMap<String, bool>,
    pub article_content: Option<String>,
    pub article_scroll: usize,
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
            feed_status: HashMap::new(),
            article_content: None,
            article_scroll: 0,
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

    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    pub fn select_last(&mut self) {
        if !self.articles.is_empty() {
            self.selected = self.articles.len() - 1;
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

    pub fn select_first_feed(&mut self) {
        self.feed_selected = 0;
    }

    pub fn select_last_feed(&mut self) {
        if !self.feeds.is_empty() {
            self.feed_selected = self.feeds.len() - 1;
        }
    }

    pub fn close_feed_list(&mut self) {
        self.input_mode = InputMode::Normal;
        self.feeds.clear();
    }

    pub fn get_feed_status(&self, url: &str) -> Option<bool> {
        self.feed_status.get(url).copied()
    }

    pub fn apply_feed_status(&mut self, status: HashMap<String, bool>) {
        self.feed_status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    const TEST_REFRESH_INTERVAL: Duration = Duration::from_secs(300);

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
        let app = App::new(articles.clone(), TEST_REFRESH_INTERVAL, false);

        assert_eq!(app.articles.len(), 3);
        assert_eq!(app.selected, 0);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_select_next() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);

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
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
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
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);

        app.quit();

        assert!(app.should_quit);
    }

    #[test]
    fn test_request_reload() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);

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
        let app = App::new(articles, TEST_REFRESH_INTERVAL, false);

        assert!(!app.should_auto_refresh());
    }

    #[test]
    fn test_update_articles_preserves_selection_if_valid() {
        let articles = create_test_articles(5);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
        app.selected = 2;

        let new_articles = create_test_articles(5);
        app.update_articles(new_articles);

        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_update_articles_adjusts_selection_if_out_of_bounds() {
        let articles = create_test_articles(5);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
        app.selected = 4;

        let new_articles = create_test_articles(2);
        app.update_articles(new_articles);

        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_selected_article() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
        app.selected = 1;

        let article = app.selected_article().unwrap();
        assert_eq!(article.title, "Article 1");
    }

    #[test]
    fn test_selected_article_empty_list() {
        let app = App::new(vec![], TEST_REFRESH_INTERVAL, false);

        assert!(app.selected_article().is_none());
    }

    #[test]
    fn test_start_adding_feed() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
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
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
        app.input_mode = InputMode::AddingFeed;
        app.input_buffer = "https://example.com".to_string();

        app.cancel_input();

        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.input_buffer.is_empty());
    }

    #[test]
    fn test_select_next_feed() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
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
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
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
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
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
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);

        app.set_status("Test message");

        assert_eq!(app.status_message, Some("Test message".to_string()));
        assert!(app.status_message_time.is_some());
    }

    #[test]
    fn test_clear_status() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
        app.set_status("Test message");

        app.clear_status();

        assert!(app.status_message.is_none());
        assert!(app.status_message_time.is_none());
    }

    #[test]
    fn test_feed_status_success() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
        let mut status = std::collections::HashMap::new();
        status.insert("https://example.com/feed.xml".to_string(), true);

        app.apply_feed_status(status);

        assert_eq!(
            app.get_feed_status("https://example.com/feed.xml"),
            Some(true)
        );
    }

    #[test]
    fn test_feed_status_failure() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, TEST_REFRESH_INTERVAL, false);
        let mut status = std::collections::HashMap::new();
        status.insert("https://example.com/feed.xml".to_string(), false);

        app.apply_feed_status(status);

        assert_eq!(
            app.get_feed_status("https://example.com/feed.xml"),
            Some(false)
        );
    }

    #[test]
    fn test_feed_status_unknown() {
        let articles = create_test_articles(3);
        let app = App::new(articles, TEST_REFRESH_INTERVAL, false);

        assert_eq!(app.get_feed_status("https://example.com/feed.xml"), None);
    }
}
