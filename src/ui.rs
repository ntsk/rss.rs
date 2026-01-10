use crate::config::Settings;
use crate::feed::{self, Article};
use crate::subscription::SubscriptionManager;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
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

pub fn run_app(articles: Vec<Article>, settings: &Settings) -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let refresh_interval = Duration::from_secs(settings.refresh_interval_secs);
    let mut app = App::new(articles, refresh_interval);
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

        if (app.should_auto_refresh() || app.should_reload)
            && let Some(new_articles) = fetch_all_articles()
        {
            app.update_articles(new_articles);
        }

        if event::poll(tick_rate)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
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
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw_ui(frame: &mut Frame, app: &App, list_state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
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

    let help = Paragraph::new("↑/↓: Navigate | Enter: Open | r: Reload | q: Quit")
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[1]);
}

fn fetch_all_articles() -> Option<Vec<Article>> {
    let config_path = crate::config::get_config_path().ok()?;
    let manager = SubscriptionManager::new(&config_path).ok()?;
    let feeds = manager.list();

    let results: Vec<_> = feeds
        .par_iter()
        .filter_map(|f| feed::fetch_articles(&f.url).ok())
        .flatten()
        .collect();

    let mut articles = results;
    articles.sort_by(|a, b| b.published.cmp(&a.published));
    Some(articles)
}

pub struct App {
    pub articles: Vec<Article>,
    pub selected: usize,
    pub should_quit: bool,
    pub should_reload: bool,
    pub last_refresh: Instant,
    pub refresh_interval: Duration,
}

impl App {
    pub fn new(articles: Vec<Article>, refresh_interval: Duration) -> Self {
        Self {
            articles,
            selected: 0,
            should_quit: false,
            should_reload: false,
            last_refresh: Instant::now(),
            refresh_interval,
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
        let app = App::new(articles.clone(), Duration::from_secs(300));

        assert_eq!(app.articles.len(), 3);
        assert_eq!(app.selected, 0);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_select_next() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300));

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
        let mut app = App::new(articles, Duration::from_secs(300));
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
        let mut app = App::new(articles, Duration::from_secs(300));

        app.quit();

        assert!(app.should_quit);
    }

    #[test]
    fn test_request_reload() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300));

        app.request_reload();

        assert!(app.should_reload);
    }

    #[test]
    fn test_should_auto_refresh_after_interval() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(0));
        app.last_refresh = Instant::now() - Duration::from_secs(1);

        assert!(app.should_auto_refresh());
    }

    #[test]
    fn test_should_not_auto_refresh_before_interval() {
        let articles = create_test_articles(3);
        let app = App::new(articles, Duration::from_secs(300));

        assert!(!app.should_auto_refresh());
    }

    #[test]
    fn test_update_articles_preserves_selection_if_valid() {
        let articles = create_test_articles(5);
        let mut app = App::new(articles, Duration::from_secs(300));
        app.selected = 2;

        let new_articles = create_test_articles(5);
        app.update_articles(new_articles);

        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_update_articles_adjusts_selection_if_out_of_bounds() {
        let articles = create_test_articles(5);
        let mut app = App::new(articles, Duration::from_secs(300));
        app.selected = 4;

        let new_articles = create_test_articles(2);
        app.update_articles(new_articles);

        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_selected_article() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300));
        app.selected = 1;

        let article = app.selected_article().unwrap();
        assert_eq!(article.title, "Article 1");
    }

    #[test]
    fn test_selected_article_empty_list() {
        let app = App::new(vec![], Duration::from_secs(300));

        assert!(app.selected_article().is_none());
    }
}
