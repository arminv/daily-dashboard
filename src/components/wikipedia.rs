use crate::{
    action::Action,
    app::LoadingStatus,
    components::Component,
    http,
    theme,
    tui::Event,
};
use crossterm::event::{
    KeyCode,
    KeyEvent,
};
use ratatui::{
    Frame,
    layout::{
        Constraint,
        Direction,
        Layout,
        Rect,
    },
    style::{
        Color,
        Modifier,
        Style,
    },
    text::{
        Line,
        Span,
    },
    widgets::{
        Block,
        List,
        ListItem,
        ListState,
        Paragraph,
        Wrap,
    },
};
use ratatui_textarea::TextArea;
use reqwest::Url;
use std::sync::{
    Arc,
    Mutex,
};
use tracing::{
    error,
    info,
};

const SEARCH_API_URL: &str = "https://en.wikipedia.org/w/api.php";
const WIKI_ORIGIN: &str = "https://en.wikipedia.org";
const MAX_RESULTS: usize = 15;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Clone, Debug, Default)]
struct WikiResult {
    title: String,
    page_id: u64,
    snippet_plain: String,
    page_url: String,
    description: Option<String>,
    extract: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct WikipediaData {
    loading_status: LoadingStatus,
    query: String,
    results: Vec<WikiResult>,
    list_state: ListState,
    extract_status: LoadingStatus,
    fetch_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnterAction {
    Search,
    Open,
}

fn enter_action(input: &str, last_query: &str, has_loaded_results: bool) -> EnterAction {
    if has_loaded_results && input.trim() == last_query {
        EnterAction::Open
    } else {
        EnterAction::Search
    }
}

fn status_paragraph(
    title: impl Into<String>,
    text: impl Into<String>,
    style: Style,
) -> Paragraph<'static> {
    Paragraph::new(text.into())
        .block(theme::panel_block(title.into()))
        .style(style)
        .wrap(Wrap { trim: true })
}

fn is_current(data: &Arc<Mutex<WikipediaData>>, generation: u64) -> bool {
    data.lock().unwrap().fetch_generation == generation
}

fn search_url(query: &str) -> String {
    let mut url = Url::parse(SEARCH_API_URL).expect("SEARCH_API_URL is valid");
    url.query_pairs_mut()
        .append_pair("action", "query")
        .append_pair("list", "search")
        .append_pair("format", "json")
        .append_pair("formatversion", "2")
        .append_pair("srlimit", &MAX_RESULTS.to_string())
        .append_pair("srsearch", query);
    url.to_string()
}

fn extracts_url(page_ids: &[u64]) -> String {
    let ids = page_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join("|");
    let mut url = Url::parse(SEARCH_API_URL).expect("SEARCH_API_URL is valid");
    url.query_pairs_mut()
        .append_pair("action", "query")
        .append_pair("format", "json")
        .append_pair("formatversion", "2")
        .append_pair("prop", "extracts|description")
        .append_pair("exintro", "1")
        .append_pair("explaintext", "1")
        .append_pair("pageids", &ids);
    url.to_string()
}

fn wiki_page_url(title: &str) -> String {
    let mut url = Url::parse(WIKI_ORIGIN).expect("WIKI_ORIGIN is valid");
    {
        let mut segments = url
            .path_segments_mut()
            .expect("WIKI_ORIGIN supports path segments");
        segments.push("wiki");
        segments.push(&title.replace(' ', "_"));
    }
    url.to_string()
}

pub struct Wikipedia {
    data: Arc<Mutex<WikipediaData>>,
    input: TextArea<'static>,
    input_mode: InputMode,
    client: reqwest::Client,
}

impl Wikipedia {
    pub fn new(client: reqwest::Client) -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default().green());
        input.set_cursor_style(Style::default().bg(Color::Red));
        input.set_placeholder_text("Search Wikipedia...");

        Self {
            data: Arc::new(Mutex::new(WikipediaData::default())),
            input,
            input_mode: InputMode::Normal,
            client,
        }
    }

    fn submit_search(&mut self) {
        let query = self.input.lines().join("").trim().to_string();
        info!("Wikipedia: submit_search called with query={:?}", query);
        if query.is_empty() {
            info!("Wikipedia: submit_search skipped — empty input");
            return;
        }

        let generation = {
            let mut data = self.data.lock().unwrap();
            if matches!(data.loading_status, LoadingStatus::Loading) {
                info!("Wikipedia: submit_search skipped — search already in flight");
                return;
            }
            data.query = query.clone();
            data.results.clear();
            data.list_state.select(Some(0));
            data.extract_status = LoadingStatus::NotStarted;
            data.loading_status = LoadingStatus::Loading;
            data.fetch_generation = data.fetch_generation.wrapping_add(1);
            data.fetch_generation
        };

        let data = self.data.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            fetch_search_results(data, query, generation, client).await;
        });
    }

    fn move_selection(&mut self, delta: i32) {
        let mut data = self.data.lock().unwrap();
        let len = data.results.len();
        if len == 0 {
            return;
        }
        let current = data.list_state.selected().unwrap_or(0);
        let next = (current as i32 + delta).clamp(0, (len - 1) as i32) as usize;
        data.list_state.select(Some(next));
    }

    fn open_selected(&self) {
        let (url, page_id, title) = {
            let data = self.data.lock().unwrap();
            let Some(idx) = data.list_state.selected() else {
                return;
            };
            match data.results.get(idx) {
                Some(r) => (r.page_url.clone(), r.page_id, r.title.clone()),
                None => return,
            }
        };
        info!("Wikipedia: opening page_id={page_id} title={title:?} url={url}");
        if let Err(e) = open::that(&url) {
            error!("Wikipedia: Failed to open URL {url}: {e}");
        }
    }

    fn handle_enter_while_editing(&mut self) {
        let input = self.input.lines().join("");
        let action = {
            let data = self.data.lock().unwrap();
            let has_results =
                matches!(data.loading_status, LoadingStatus::Loaded) && !data.results.is_empty();
            enter_action(&input, &data.query, has_results)
        };
        match action {
            EnterAction::Open => self.open_selected(),
            EnterAction::Search => self.submit_search(),
        }
    }

    fn render_input(&mut self, frame: &mut Frame, area: Rect) {
        let (title, title_style) = match self.input_mode {
            InputMode::Normal => (
                "📚 Wikipedia — press / to search",
                Style::default().fg(theme::ACCENT),
            ),
            InputMode::Editing => (
                "📚 Wikipedia — ↑/↓ results · Enter search/open · Esc done",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        };
        self.input.set_block(
            Block::default()
                .title(title)
                .style(title_style)
                .borders(ratatui::widgets::Borders::ALL),
        );
        frame.render_widget(&self.input, area);
    }

    fn render_results(&mut self, frame: &mut Frame, area: Rect) {
        let mut data = self.data.lock().unwrap();

        match &data.loading_status {
            LoadingStatus::NotStarted => {
                frame.render_widget(
                    status_paragraph(
                        "Results",
                        "Press / to type a query, then Enter to search.",
                        Style::default().fg(theme::HINT),
                    ),
                    area,
                );
            }
            LoadingStatus::Loading => {
                let text = format!("Searching \"{}\"...", data.query);
                frame.render_widget(
                    status_paragraph("Results", text, Style::default().fg(theme::LOADING)),
                    area,
                );
            }
            LoadingStatus::Error(error) => {
                let text = format!("No results for \"{}\"\n\n{error}", data.query);
                frame.render_widget(
                    status_paragraph("Results", text, Style::default().fg(theme::ERROR)),
                    area,
                );
            }
            LoadingStatus::Loaded => {
                let selected = data.list_state.selected().unwrap_or(0);
                let title = format!("Results ({}/{})", selected + 1, data.results.len());
                let items: Vec<ListItem> = data
                    .results
                    .iter()
                    .map(|result| {
                        let mut lines = vec![Line::from(result.title.clone())];
                        if !result.snippet_plain.is_empty() {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", result.snippet_plain),
                                Style::default().fg(theme::HINT),
                            )));
                        }
                        ListItem::new(lines)
                    })
                    .collect();
                let list = List::new(items)
                    .block(theme::panel_block(title))
                    .highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, area, &mut data.list_state);
            }
        }
    }

    fn render_extract(&self, frame: &mut Frame, area: Rect) {
        let data = self.data.lock().unwrap();
        let selected = data
            .list_state
            .selected()
            .and_then(|idx| data.results.get(idx));
        let title = selected
            .map(|r| format!("Extract — {}", r.title))
            .unwrap_or_else(|| "Extract".to_string());

        match (&data.loading_status, &data.extract_status) {
            (LoadingStatus::Loaded, LoadingStatus::Loading) => {
                let body = selected
                    .map(|r| r.snippet_plain.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| format!("Loading full extract...\n\n{s}"))
                    .unwrap_or_else(|| "Loading extracts...".to_string());
                frame.render_widget(
                    status_paragraph(title, body, Style::default().fg(theme::LOADING)),
                    area,
                );
            }
            (LoadingStatus::Loaded, LoadingStatus::Error(error)) => {
                frame.render_widget(
                    status_paragraph(title, error.clone(), Style::default().fg(theme::ERROR)),
                    area,
                );
            }
            (LoadingStatus::Loaded, LoadingStatus::Loaded) => {
                let hint = Line::from(Span::styled(
                    "↑/↓ move · Enter open (same query) or search",
                    Style::default().fg(theme::HINT),
                ));
                let mut lines = vec![hint, Line::from("")];
                if let Some(result) = selected {
                    if let Some(description) = &result.description {
                        lines.push(Line::from(Span::styled(
                            description.clone(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::ITALIC),
                        )));
                        lines.push(Line::from(""));
                    }
                    let extract = result
                        .extract
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(result.snippet_plain.as_str());
                    for paragraph in extract.split("\n\n") {
                        lines.push(Line::from(paragraph.to_string()));
                        lines.push(Line::from(""));
                    }
                }
                let paragraph = Paragraph::new(lines)
                    .block(theme::panel_block(title))
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
            _ => {
                frame.render_widget(
                    status_paragraph(
                        title,
                        "Select a result to read its extract.",
                        Style::default().fg(theme::HINT),
                    ),
                    area,
                );
            }
        }
    }
}

async fn fetch_search_results(
    data: Arc<Mutex<WikipediaData>>,
    query: String,
    generation: u64,
    client: reqwest::Client,
) {
    let api_url = search_url(&query);
    info!("Wikipedia: fetching search {api_url}");

    let json: serde_json::Value = match http::get_json(&client, &api_url).await {
        Ok(json) => json,
        Err(e) => {
            if !is_current(&data, generation) {
                return;
            }
            data.lock().unwrap().loading_status = LoadingStatus::from_report("Wikipedia", &e);
            return;
        }
    };

    if !is_current(&data, generation) {
        return;
    }

    let results = parse_search_results(&json);
    if results.is_empty() {
        if !is_current(&data, generation) {
            return;
        }
        data.lock().unwrap().loading_status =
            LoadingStatus::from_msg("Wikipedia", format!("No results for \"{query}\""));
        return;
    }

    let page_ids: Vec<u64> = results.iter().map(|r| r.page_id).collect();
    {
        let mut state = data.lock().unwrap();
        if state.fetch_generation != generation {
            return;
        }
        state.results = results;
        state.list_state.select(Some(0));
        state.loading_status = LoadingStatus::Loaded;
        state.extract_status = LoadingStatus::Loading;
        info!(
            "Wikipedia: loaded {} results for {:?}",
            state.results.len(),
            state.query
        );
    }

    fetch_extracts_batch(data, page_ids, generation, client).await;
}

async fn fetch_extracts_batch(
    data: Arc<Mutex<WikipediaData>>,
    page_ids: Vec<u64>,
    generation: u64,
    client: reqwest::Client,
) {
    if page_ids.is_empty() {
        return;
    }
    let api_url = extracts_url(&page_ids);
    info!(
        "Wikipedia: fetching extracts batch ({}) {api_url}",
        page_ids.len()
    );

    let json: serde_json::Value = match http::get_json(&client, &api_url).await {
        Ok(json) => json,
        Err(e) => {
            if !is_current(&data, generation) {
                return;
            }
            data.lock().unwrap().extract_status =
                LoadingStatus::from_report("Wikipedia extract", &e);
            return;
        }
    };

    let mut state = data.lock().unwrap();
    if state.fetch_generation != generation {
        return;
    }
    apply_extracts_from_query(&mut state.results, &json);
    state.extract_status = LoadingStatus::Loaded;
}

fn parse_search_results(json: &serde_json::Value) -> Vec<WikiResult> {
    let Some(search) = json.pointer("/query/search").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    search
        .iter()
        .filter_map(|item| {
            let title = item.get("title")?.as_str()?.to_string();
            let page_id = item.get("pageid")?.as_u64()?;
            let snippet_html = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            let snippet_plain = strip_search_html(snippet_html);
            let page_url = wiki_page_url(&title);
            let extract = (!snippet_plain.is_empty()).then_some(snippet_plain.clone());
            Some(WikiResult {
                title,
                page_id,
                snippet_plain,
                page_url,
                description: None,
                extract,
            })
        })
        .take(MAX_RESULTS)
        .collect()
}

/// Fill `description` / `extract` on matching results from `prop=extracts|description`.
fn apply_extracts_from_query(results: &mut [WikiResult], json: &serde_json::Value) {
    let Some(pages) = json.pointer("/query/pages").and_then(|v| v.as_array()) else {
        return;
    };
    for page in pages {
        let Some(page_id) = page.get("pageid").and_then(|v| v.as_u64()) else {
            continue;
        };
        let Some(extract) = page
            .get("extract")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let description = page
            .get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        if let Some(result) = results.iter_mut().find(|r| r.page_id == page_id) {
            result.description = description;
            result.extract = Some(extract.to_string());
        }
    }
}

fn strip_search_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#039;", "'")
        .replace("&nbsp;", " ")
}

fn is_search_activation_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('/') && key.modifiers.is_empty()
}

impl Component for Wikipedia {
    fn is_capturing_input(&self) -> bool {
        self.input_mode == InputMode::Editing
    }

    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<Option<Action>> {
        let Some(Event::Key(key)) = event else {
            return Ok(None);
        };

        match self.input_mode {
            InputMode::Normal => {
                if is_search_activation_key(key) {
                    self.input_mode = InputMode::Editing;
                    return Ok(Some(Action::Render));
                }
                Ok(None)
            }
            InputMode::Editing => {
                match key.code {
                    KeyCode::Esc => self.input_mode = InputMode::Normal,
                    KeyCode::Enter => self.handle_enter_while_editing(),
                    KeyCode::Up => self.move_selection(-1),
                    KeyCode::Down => self.move_selection(1),
                    _ => {
                        self.input.input(key);
                    }
                };
                Ok(Some(Action::Render))
            }
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if action == Action::Tick {
            let cursor_color = if self.input_mode == InputMode::Editing {
                Color::Red
            } else {
                Color::Blue
            };
            self.input
                .set_cursor_style(Style::default().bg(cursor_color));
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Ratio(1, 3),
                Constraint::Min(1),
            ])
            .split(area);
        self.render_input(frame, chunks[0]);
        self.render_results(frame, chunks[1]);
        self.render_extract(frame, chunks[2]);
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/wikipedia.rs"]
mod tests;
