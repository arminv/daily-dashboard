use super::Component;
use crate::{
    action::Action,
    app::LoadingStatus,
    http,
    theme,
};
use color_eyre::Result;
use crossterm::event::{
    KeyCode,
    KeyEvent,
};
use image::DynamicImage;
use ratatui::{
    Frame,
    layout::{
        Constraint,
        Direction,
        Layout,
        Rect,
    },
    style::Style,
    widgets::{
        Paragraph,
        Wrap,
    },
};
use ratatui_image::{
    Resize,
    StatefulImage,
    picker::{
        Picker,
        ProtocolType,
    },
    thread::{
        ResizeRequest,
        ThreadProtocol,
    },
};
use std::sync::{
    Arc,
    Mutex,
};
use tokio::sync::mpsc::{
    UnboundedReceiver,
    unbounded_channel,
};
use tracing::{
    error,
    info,
    warn,
};

const PICSUM_BASE_URL: &str = "https://picsum.photos";
const PICSUM_WIDTH_PX: u32 = 1200;
const PICSUM_HEIGHT_PX: u32 = 800;

#[derive(Debug, Default)]
pub struct ImageState {
    pub loading_status: LoadingStatus,
    pub pending_image: Option<DynamicImage>,
    pub is_in_flight: bool,
    pub is_refetch_requested: bool,
}

pub struct PictureFrame {
    state: Arc<Mutex<ImageState>>,
    client: reqwest::Client,
    picker: Picker,
    resize_rx: UnboundedReceiver<ResizeRequest>,
    protocol: ThreadProtocol,
}

impl PictureFrame {
    /// Build the image-protocol picker, honoring `DAILY_DASHBOARD_IMAGE_PROTOCOL`:
    /// `halfblocks` (works everywhere, low fidelity), `kitty`/`sixel`/`iterm2`
    /// (force a specific protocol), or unset/`auto` for terminal auto-detection
    /// (see `auto_picker`). Anything unrecognized falls back to halfblocks.
    pub fn build_picker() -> Picker {
        let configured = std::env::var("DAILY_DASHBOARD_IMAGE_PROTOCOL")
            .ok()
            .map(|s| s.to_lowercase());

        match configured.as_deref() {
            Some("halfblocks") | Some("halfblock") => Picker::halfblocks(),
            Some("kitty") => force_protocol(ProtocolType::Kitty),
            Some("sixel") => force_protocol(ProtocolType::Sixel),
            Some("iterm2") | Some("iterm") => force_protocol(ProtocolType::Iterm2),
            Some("auto") => auto_picker(),
            Some(other) => {
                warn!(
                    "Unknown DAILY_DASHBOARD_IMAGE_PROTOCOL=`{other}`, falling back to halfblocks"
                );
                Picker::halfblocks()
            }
            None => auto_picker(),
        }
    }

    pub fn new(client: reqwest::Client, picker: Picker) -> Self {
        // ThreadProtocol offloads the heavy resize+encode work off the render
        // path: it sends ResizeRequests through this channel, which we drain in
        // `update()` and feed the encoded result back via
        // `update_resized_protocol`.
        let (resize_tx, resize_rx) = unbounded_channel::<ResizeRequest>();
        let protocol = ThreadProtocol::new(resize_tx, None);
        Self {
            state: Arc::new(Mutex::new(ImageState::default())),
            client,
            picker,
            resize_rx,
            protocol,
        }
    }

    fn maybe_spawn_fetch(&mut self) {
        let should_spawn = {
            let state = self.state.lock().unwrap();
            !state.is_in_flight
                && (state.is_refetch_requested
                    || matches!(state.loading_status, LoadingStatus::NotStarted))
        };

        if should_spawn {
            {
                let mut state = self.state.lock().unwrap();
                state.is_in_flight = true;
                state.is_refetch_requested = false;
                if matches!(state.loading_status, LoadingStatus::NotStarted) {
                    state.loading_status = LoadingStatus::Loading;
                }
            }
            let client = self.client.clone();
            let state = self.state.clone();
            tokio::spawn(async move {
                fetch_image(client, state).await;
            });
        }
    }

    /// Drain pending resize requests and feed the encoded results back into the
    /// protocol. Called from `update()` (i.e. between renders), so the expensive
    /// encode never blocks `draw()`.
    fn poll_resize_requests(&mut self) {
        while let Ok(request) = self.resize_rx.try_recv() {
            match request.resize_encode() {
                Ok(response) => {
                    self.protocol.update_resized_protocol(response);
                }
                Err(e) => error!("Daily Picture: resize_encode failed: {e}"),
            }
        }
    }

    /// Install the pending image into the protocol. Called from `update()` (i.e.
    /// between renders), so the expensive encode never blocks `draw()`.
    fn install_pending_image(&mut self) {
        let pending = self.state.lock().unwrap().pending_image.take();
        if let Some(image) = pending {
            let protocol = self.picker.new_resize_protocol(image);
            self.protocol.replace_protocol(protocol);
        }
    }
}

impl Component for PictureFrame {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if is_new_image_key(&key) {
            self.state.lock().unwrap().is_refetch_requested = true;
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if action == Action::Tick {
            self.maybe_spawn_fetch();
        }
        self.poll_resize_requests();
        self.install_pending_image();
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let status = self.state.lock().unwrap().loading_status.clone();

        if let LoadingStatus::Loaded = &status {
            let block = theme::frame_block("🖼  Daily Picture");
            let inner = block.inner(area);
            let widget = StatefulImage::new().resize(Resize::Fit(None));
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(2)])
                .split(inner);
            frame.render_stateful_widget(widget, chunks[0], &mut self.protocol);
            frame.render_widget(hint_paragraph(), chunks[1]);
            frame.render_widget(block, area);
            return Ok(());
        }

        let block = match &status {
            LoadingStatus::Error(error) => theme::panel_block_colored(
                format!("🖼  Daily Picture - Error: {error}"),
                theme::ERROR,
            ),
            LoadingStatus::Loading => theme::panel_block("🖼  Daily Picture - Loading..."),
            _ => theme::panel_block("🖼  Daily Picture"),
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(hint_paragraph(), inner);
        Ok(())
    }
}

/// Force a specific graphics protocol while still detecting the font size via
/// `from_query_stdio`. Falls back to halfblocks if the stdio query fails
/// (graphics protocols can't map cells without a pixel font size).
fn force_protocol(protocol_type: ProtocolType) -> Picker {
    match Picker::from_query_stdio() {
        Ok(mut picker) => {
            picker.set_protocol_type(protocol_type);
            info!(
                "Picture frame graphics protocol: forced {:?}",
                picker.protocol_type()
            );
            picker
        }
        Err(_) => Picker::halfblocks(),
    }
}

/// Auto-detect the font size via `from_query_stdio`, then override the protocol
/// from `TERM_PROGRAM` for well-known terminals that report misleading graphics
/// capabilities.
///
/// The IO query can misdetect Kitty: iTerm2 answers it but only renders its own
/// protocol, and Warp implements Kitty *graphics* but not Kitty Unicode
/// placeholders (so `Resize::Fit` would emit `[?]` tofu) - both render fine via
/// iTerm2 (OSC 1337), so we force iTerm2 there. Trusting `TERM_PROGRAM` for
/// these is more reliable than the query.
fn auto_picker() -> Picker {
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

    // VS Code / Cursor integrated terminal (xterm.js): inline-image support
    // (`terminal.integrated.enableImages`) is off by default yet the terminal
    // still answers capability queries, so `from_query_stdio` can pick a
    // graphics protocol that renders nothing. Halfblocks always render; users
    // who enabled images can override via `DAILY_DASHBOARD_IMAGE_PROTOCOL`.
    if term_program.contains("vscode") {
        info!(
            "Picture frame graphics protocol: Halfblocks (VS Code/Cursor terminal; \
             set DAILY_DASHBOARD_IMAGE_PROTOCOL=iterm2 to override if image support is enabled)"
        );
        return Picker::halfblocks();
    }

    let mut picker = match Picker::from_query_stdio() {
        Ok(picker) => picker,
        Err(_) => return Picker::halfblocks(),
    };

    let forced = if term_program.contains("iTerm")
        || term_program.contains("WezTerm")
        || std::env::var("WEZTERM_EXECUTABLE").is_ok()
        || term_program.contains("Warp")
    {
        Some(ProtocolType::Iterm2)
    } else if term_program.contains("kitty")
        || term_program == "ghostty"
        || std::env::var("KITTY_WINDOW_ID").is_ok()
    {
        Some(ProtocolType::Kitty)
    } else {
        None
    };
    if let Some(protocol_type) = forced {
        picker.set_protocol_type(protocol_type);
    }

    info!(
        "Picture frame graphics protocol: {:?} (TERM_PROGRAM={:?})",
        picker.protocol_type(),
        if term_program.is_empty() {
            "<unset>"
        } else {
            &term_program
        }
    );
    picker
}

fn record_error(state: &Arc<Mutex<ImageState>>, err: color_eyre::Report) {
    let status = LoadingStatus::from_report("Daily Picture", &err);
    let mut state = state.lock().unwrap();
    state.is_in_flight = false;
    // Keep showing the last-good image if one was already loaded; only surface
    // the error when there is nothing else to display.
    if !matches!(state.loading_status, LoadingStatus::Loaded) {
        state.loading_status = status;
    }
}

async fn fetch_image(client: reqwest::Client, state: Arc<Mutex<ImageState>>) {
    let url = image_url();

    let bytes = match http::get_bytes_redirected(&client, &url).await {
        Ok((bytes, _)) => bytes,
        Err(e) => {
            record_error(&state, e.wrap_err("failed to download image"));
            return;
        }
    };

    let image = match image::load_from_memory(&bytes) {
        Ok(image) => image,
        Err(e) => {
            record_error(
                &state,
                color_eyre::eyre::eyre!("failed to decode image: {e}"),
            );
            return;
        }
    };

    let mut state = state.lock().unwrap();
    state.pending_image = Some(image);
    state.is_in_flight = false;
    state.loading_status = LoadingStatus::Loaded;
}

pub(crate) fn image_url() -> String {
    format!("{PICSUM_BASE_URL}/{PICSUM_WIDTH_PX}/{PICSUM_HEIGHT_PX}")
}

/// Does this key event request a new image (Shift+N)?
///
/// We run in legacy terminal mode (the app never enables the Kitty keyboard
/// protocol), where there is no separate "shift" bit on the wire: Shift+n is
/// transmitted *as* the uppercase character `N`. crossterm even derives
/// `KeyModifiers::SHIFT` from that uppercase-ness, so matching the char is the
/// semantic check - also testing the modifier would just re-test the same thing.
/// A plain lowercase `n`, and any `Ctrl`/`Ctrl+Shift` combo (which arrives as a
/// control byte, i.e. lowercase `Ctrl+n`), are correctly ignored. The only
/// ambiguity is that Caps Lock + `n` also yields `N`, which is harmless here.
/// Pure (no I/O) so it can be unit-tested.
pub(crate) fn is_new_image_key(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('N')
}

fn hint_paragraph() -> Paragraph<'static> {
    Paragraph::new("Shift+N - fetch a new image")
        .style(Style::default().fg(theme::HINT))
        .wrap(Wrap { trim: true })
}

#[cfg(test)]
#[path = "../tests/picture_frame.rs"]
mod tests;
