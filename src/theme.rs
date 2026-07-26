use ratatui::{
    Frame,
    layout::Rect,
    style::{
        Color,
        Style,
    },
    text::Line,
    widgets::{
        Block,
        Borders,
        Paragraph,
        Wrap,
    },
};

/// Primary accent color used for panel borders and titles throughout the app.
pub const ACCENT: Color = Color::Cyan;

/// Secondary status/content accent colors shared across widgets.
pub const LOADING: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const HINT: Color = Color::DarkGray;

/// A bordered panel block for widgets that draw their own content into the
/// block's inner area (weather, news, inspiration, dictionary definition).
///
/// Uses `.style()` so the accent color reaches empty cells too; this is safe
/// because the widget renders its content into `block.inner(area)` afterwards,
/// overriding those cells with its own styles.
pub fn panel_block<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    panel_block_colored(title, ACCENT)
}

/// Like [`panel_block`] but with an explicit border/title color, used for
/// status states such as loading (see [`LOADING`]) and error (see [`ERROR`]).
pub fn panel_block_colored<'a>(title: impl Into<Line<'a>>, color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(color))
}

/// A bordered frame block for the shared Calendar + Greeting panel, where the
/// block is drawn *after* the calendar content. Uses only `.border_style()` and
/// `.title_style()` (no `.style()`) so the already-drawn calendar is not
/// recolored — `Block::render` calls `buf.set_style(area, self.style)` over the
/// entire area, which would wipe the calendar's red highlight.
pub fn frame_block<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(ACCENT))
        .border_style(Style::default().fg(ACCENT))
}

/// Render a bordered status panel with an optional body message.
///
/// Used for NotStarted / Loading / Error empty states where the border color
/// and title carry the status (Inspiration, News, Daily Picture) or stay on
/// [`ACCENT`] while the body text carries it (Dictionary). An empty `message`
/// draws only the bordered frame.
pub fn render_status_panel(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<Line<'static>>,
    border_color: Color,
    message: impl AsRef<str>,
    message_color: Color,
) {
    let block = panel_block_colored(title, border_color);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let message = message.as_ref();
    if message.is_empty() {
        return;
    }
    frame.render_widget(
        Paragraph::new(message.to_string())
            .style(Style::default().fg(message_color))
            .wrap(Wrap { trim: true }),
        inner,
    );
}

/// Build a status paragraph whose **body** carries the status color, while the
/// border stays on [`ACCENT`] (Wikipedia results / extract panes).
pub fn status_paragraph(
    title: impl Into<String>,
    message: impl Into<String>,
    message_color: Color,
) -> Paragraph<'static> {
    Paragraph::new(message.into())
        .block(panel_block(title.into()))
        .style(Style::default().fg(message_color))
        .wrap(Wrap { trim: true })
}
