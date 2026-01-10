use crate::feed::Article;
use anyhow::Result;
use std::time::{Duration, Instant};

const REFRESH_INTERVAL_SECS: u64 = 300;

pub fn run_app(_articles: Vec<Article>) -> Result<()> {
    todo!()
}

pub struct App {
    pub articles: Vec<Article>,
    pub selected: usize,
    pub should_quit: bool,
    pub should_open: bool,
    pub should_reload: bool,
    pub last_refresh: Instant,
    pub refresh_interval: Duration,
}

impl App {
    pub fn new(articles: Vec<Article>, refresh_interval: Duration) -> Self {
        todo!()
    }

    pub fn select_next(&mut self) {
        todo!()
    }

    pub fn select_previous(&mut self) {
        todo!()
    }

    pub fn open_selected(&mut self) {
        todo!()
    }

    pub fn quit(&mut self) {
        todo!()
    }

    pub fn request_reload(&mut self) {
        todo!()
    }

    pub fn should_auto_refresh(&self) -> bool {
        todo!()
    }

    pub fn update_articles(&mut self, articles: Vec<Article>) {
        todo!()
    }

    pub fn selected_article(&self) -> Option<&Article> {
        todo!()
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
        assert!(!app.should_open);
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
    fn test_open_selected() {
        let articles = create_test_articles(3);
        let mut app = App::new(articles, Duration::from_secs(300));

        app.open_selected();

        assert!(app.should_open);
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
