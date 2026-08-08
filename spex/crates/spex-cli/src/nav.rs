//! `spex nav` — a k9s-style interactive browser over `discover_demos()`'s
//! output: move through the list, view a demo's tree inline, or launch its
//! web view, without re-typing a path into a fresh command each time.
use crate::{discover_demos, DemoEntry};
#[cfg(test)]
use crate::DemoKind;
use ansi_to_tui::IntoText;
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, Terminal};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Mirrors `spex ascii --animate`'s own defaults (`main.rs`'s `Ascii` clap
/// command) so the live preview looks the same as running that command
/// directly against the same tileset.
const ANIMATION_WIDTH: usize = 100;
const ANIMATION_FRAME_COUNT: usize = 24;
/// Redraw cadence for the live preview loop — matches `spex ascii --animate`'s
/// default `--fps 10`, and doubles as the `event::poll` timeout that lets
/// `run_app` redraw on a timer instead of only on a keypress.
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(100);

enum Mode {
    List,
    Detail { text: String, scroll: u16 },
    /// A live turntable-orbit ASCII animation (same technique as `spex ascii
    /// --animate`, see `build_animation_frames`), redrawn on a timer inside
    /// the detail pane. Keeps the underlying static view's text/scroll so
    /// toggling animation off returns to exactly where the user left it,
    /// rather than recomputing `format_tree()`'s output a second time.
    Animate { frames: Vec<String>, frame: usize, detail_text: String, detail_scroll: u16 },
}

struct App {
    demos: Vec<DemoEntry>,
    selected: usize,
    mode: Mode,
    status: Option<String>,
    /// `/`-filter on the demo list (name or title, case-insensitive
    /// substring). Empty means unfiltered — every demo shown.
    filter: String,
    /// Whether `/` is actively being typed into right now (a lightweight
    /// modal state, not a `Mode` variant, since it only ever applies on top
    /// of `Mode::List` — `Mode::Detail` has no filter of its own).
    filtering: bool,
}

impl App {
    fn new(demos: Vec<DemoEntry>) -> Self {
        App {
            demos,
            selected: 0,
            mode: Mode::List,
            status: None,
            filter: String::new(),
            filtering: false,
        }
    }

    /// Indices into `self.demos` matching the current filter, preserving
    /// original order. All indices when the filter is empty.
    fn filtered_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.demos.len()).collect();
        }
        let query = self.filter.to_lowercase();
        self.demos
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                d.name.to_lowercase().contains(&query) || d.title.as_deref().unwrap_or("").to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn selected_demo(&self) -> Option<&DemoEntry> {
        let indices = self.filtered_indices();
        indices.get(self.selected).and_then(|&i| self.demos.get(i))
    }
}

pub fn run(dir: &Path) -> Result<()> {
    let demos = discover_demos(dir)?;
    if demos.is_empty() {
        println!("no demos found in {} yet", dir.display());
        println!("try: ./scripts/walkthrough.sh  (generates a handful of example demos)");
        return Ok(());
    }

    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, App::new(demos));
    restore_terminal(&mut terminal);
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    install_panic_hook();
    enable_raw_mode().context("enabling raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("entering alternate screen")?;
    Terminal::new(CrosstermBackend::new(stdout)).context("creating terminal")
}

/// Always leaves the terminal as we found it, even if `run_app` returned an
/// error — a bug here would otherwise leave the user's real shell broken
/// (stuck in raw mode / the alternate screen).
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

/// Same as `restore_terminal`, but reachable from a panic hook (which can't
/// borrow a `Terminal`) — a panic inside the render/event loop must not
/// leave raw mode / the alternate screen active either.
fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original(info);
    }));
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| render(f, &app))?;

        // Every other mode is fine blocking on event::read() like before —
        // Mode::Animate is the one case that needs to redraw with no
        // keypress at all. event::poll's bounded timeout doubles as the
        // animation's frame clock: no event within one frame interval means
        // "advance and redraw", not "nothing happened".
        if let Mode::Animate { frames, frame, .. } = &mut app.mode {
            if !event::poll(ANIMATION_FRAME_INTERVAL)? {
                *frame = (*frame + 1) % frames.len();
                continue;
            }
        }

        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if matches!(app.mode, Mode::Animate { .. }) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('a') | KeyCode::Char('q') => {
                    if let Mode::Animate { detail_text, detail_scroll, .. } = &app.mode {
                        let (text, scroll) = (detail_text.clone(), *detail_scroll);
                        app.mode = Mode::Detail { text, scroll };
                    }
                }
                _ => {}
            }
        } else if matches!(app.mode, Mode::Detail { .. }) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => app.mode = Mode::List,
                KeyCode::Char('a') => {
                    if let Mode::Detail { text, scroll } = &app.mode {
                        let (text, scroll) = (text.clone(), *scroll);
                        if let Some(demo) = app.selected_demo() {
                            match build_animation_frames(demo) {
                                Ok(frames) => {
                                    app.status = None;
                                    app.mode = Mode::Animate { frames, frame: 0, detail_text: text, detail_scroll: scroll };
                                }
                                Err(e) => app.status = Some(e),
                            }
                        }
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Mode::Detail { scroll, .. } = &mut app.mode {
                        *scroll = scroll.saturating_sub(1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Mode::Detail { scroll, .. } = &mut app.mode {
                        *scroll = scroll.saturating_add(1);
                    }
                }
                _ => {}
            }
        } else if app.filtering {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => app.filtering = false,
                KeyCode::Backspace => {
                    app.filter.pop();
                    app.selected = 0;
                }
                KeyCode::Char(c) => {
                    app.filter.push(c);
                    app.selected = 0;
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('/') => app.filtering = true,
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.selected > 0 {
                        app.selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.selected + 1 < app.filtered_indices().len() {
                        app.selected += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('v') => {
                    if let Some(demo) = app.selected_demo() {
                        // format_tree()'s color gating checks stdout.is_terminal(),
                        // which is true here (nav only ever runs attached to a
                        // real terminal), so this already carries real ANSI
                        // truecolor codes — render_detail() converts them to
                        // real ratatui styling via ansi_to_tui. Point-cloud/
                        // sequence demos have no real tree to show at all —
                        // say so plainly rather than erroring.
                        let text = match &demo.graph_path {
                            Some(graph_path) => spex_graph::Graph::read_json(graph_path)
                                .map(|g| spex_graph::format_tree(&g))
                                .unwrap_or_else(|e| format!("failed to read {}: {e}", graph_path.display())),
                            None => format!("{} is a point cloud/animation, not a graph — no tree to show. Press 'w' to view it in the browser.", demo.name),
                        };
                        app.mode = Mode::Detail { text, scroll: 0 };
                    }
                }
                KeyCode::Char('w') => {
                    if let Some(demo) = app.selected_demo() {
                        app.status = Some(open_web_view(demo));
                    }
                }
                _ => {}
            }
        }
    }
}

/// Deterministic per demo name so re-opening the same demo reuses its port
/// instead of colliding with itself; two different demos could still
/// collide on a hash collision within this 20-port range, which is an
/// accepted v1 limitation (see docs/ARCHITECTURE.md-style plan notes).
fn port_for(name: &str) -> u16 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    8080 + (hasher.finish() % 20) as u16
}

/// Launches `spex serve` as a detached subprocess rather than running the
/// server in-process: sidesteps coordinating an async runtime with the
/// synchronous TUI event loop, and a crash in the server can't take the
/// navigator down with it.
fn open_web_view(demo: &DemoEntry) -> String {
    if !demo.web_ready {
        // Only a Graph demo can reach this not-web-ready state today (a
        // PointCloud/Sequence demo is only ever discovered once its real
        // tileset/sequence already exists) — graph_path is real here.
        let graph_path = demo.graph_path.as_deref().map(|p| p.display().to_string()).unwrap_or_default();
        return format!(
            "{} has no tileset yet — run `spex graph-layout {} -o {}` first",
            demo.name,
            graph_path,
            demo.tileset_dir.display()
        );
    }
    let port = port_for(&demo.name);
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return format!("couldn't find spex binary: {e}"),
    };
    match Command::new(exe)
        .arg("serve")
        .arg(&demo.tileset_dir)
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => format!("opened {} at http://127.0.0.1:{port}/", demo.name),
        Err(e) => format!("failed to launch web view: {e}"),
    }
}

/// Loads `demo`'s real tileset points and projects them into a turntable
/// orbit of ANSI-colored frames — the exact same `ascii::render_frames`
/// `spex ascii --animate` uses to build its own animation, just consumed
/// live inside the TUI instead of printed to stdout. Returns a status
/// message on failure instead of an error type, matching `open_web_view`'s
/// convention for surfacing a problem in the footer rather than crashing the
/// navigator.
fn build_animation_frames(demo: &DemoEntry) -> Result<Vec<String>, String> {
    if !demo.web_ready {
        return Err(format!("{} has no tileset yet — nothing to animate", demo.name));
    }
    let points = spex_tiler::read_points(&demo.tileset_dir).map_err(|e| format!("failed to read {}: {e}", demo.tileset_dir.display()))?;
    // `None`: no packet-path overlay in the nav preview (yet) — this just
    // needs to restore the plain turntable-orbit animation `render_frames`
    // already supported before issue #9 added the optional 4th parameter;
    // a real follow-up could pass this demo's own nodes.json path data
    // through, same as `spex ascii --animate` now can.
    let frames = crate::ascii::render_frames(&points, ANIMATION_WIDTH, ANIMATION_FRAME_COUNT, None);
    if frames.is_empty() {
        return Err(format!("{} has no visible points to animate", demo.name));
    }
    Ok(frames)
}

fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    match &app.mode {
        Mode::List => render_list(f, chunks[0], app),
        Mode::Detail { text, scroll } => render_detail(f, chunks[0], app, text, *scroll),
        Mode::Animate { frames, frame, .. } => render_animate(f, chunks[0], app, &frames[*frame]),
    }
    render_footer(f, chunks[1], app);
}

fn render_list(f: &mut Frame, area: Rect, app: &App) {
    let indices = app.filtered_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .map(|&i| {
            let d = &app.demos[i];
            let title = d.title.clone().unwrap_or_else(|| d.name.clone());
            let ready = if d.web_ready { "" } else { "  (no tileset yet)" };
            ListItem::new(format!("{:<16} {:>4} nodes   {title}{ready}", d.name, d.node_count))
        })
        .collect();

    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.selected.min(items.len() - 1)));
    }

    let title = if app.filter.is_empty() {
        " spex nav — demos ".to_string()
    } else {
        format!(" spex nav — demos ({}/{} matching \"{}\") ", items.len(), app.demos.len(), app.filter)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(list, area, &mut state);
}

fn render_detail(f: &mut Frame, area: Rect, app: &App, text: &str, scroll: u16) {
    let name = app.selected_demo().map(|d| d.name.as_str()).unwrap_or("");
    // `text` already carries real ANSI truecolor codes from `format_tree()`
    // (see the Enter/'v' handler) — parse them into real ratatui styling
    // instead of showing plain text or raw escape codes. Malformed/unknown
    // sequences are ignored by the parser, and a `NO_COLOR`/non-tty run
    // (which `format_tree` itself would have already rendered plain) falls
    // back to plain text the same way.
    let content = text.into_text().unwrap_or_else(|_| ratatui::text::Text::raw(text));
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(format!(" {name} ")))
        .scroll((scroll, 0));
    f.render_widget(paragraph, area);
}

fn render_animate(f: &mut Frame, area: Rect, app: &App, frame_text: &str) {
    let name = app.selected_demo().map(|d| d.name.as_str()).unwrap_or("");
    // Same ANSI-to-ratatui-styling path as render_detail — ascii::render_frames
    // produces real ANSI truecolor codes just like format_tree() does.
    let content = frame_text.into_text().unwrap_or_else(|_| ratatui::text::Text::raw(frame_text));
    let paragraph = Paragraph::new(content).block(Block::default().borders(Borders::ALL).title(format!(" {name} — turntable preview ")));
    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let text = if app.filtering {
        format!("/{}\u{2588}   (enter/esc: done filtering)", app.filter)
    } else {
        let help = match app.mode {
            Mode::List => "\u{2191}/k \u{2193}/j: move   enter/v: view tree   w: open web view   /: filter   q: quit",
            Mode::Detail { .. } => "\u{2191}/k \u{2193}/j: scroll   a: animate   esc/q: back",
            Mode::Animate { .. } => "a/esc: stop animation",
        };
        match &app.status {
            Some(s) => format!("{help}   |   {s}"),
            None => help.to_string(),
        }
    };
    f.render_widget(Paragraph::new(text).style(Style::default().add_modifier(Modifier::DIM)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use serde_json::Map;
    use spex_graph::{Graph, GraphNode};

    fn demo(name: &str, title: &str, node_count: usize, web_ready: bool) -> DemoEntry {
        DemoEntry {
            name: name.to_string(),
            title: Some(title.to_string()),
            kind: DemoKind::Graph,
            graph_path: Some(Path::new("/tmp/does-not-matter/graph.json").to_path_buf()),
            tileset_dir: Path::new("/tmp/does-not-matter/tileset").to_path_buf(),
            node_count,
            web_ready,
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn list_view_shows_demo_names_and_marks_not_ready() {
        let demos = vec![demo("decix-trace", "traceroute to www.de-cix.net", 10, true), demo("fresh-capture", "just captured", 3, false)];
        let app = App::new(demos);
        let mut terminal = Terminal::new(TestBackend::new(90, 10)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("decix-trace"), "{text}");
        assert!(text.contains("traceroute to www.de-cix.net"), "{text}");
        assert!(text.contains("fresh-capture"), "{text}");
        assert!(text.contains("no tileset yet"), "{text}");
        assert!(text.contains("open web view"), "{text}"); // footer help
    }

    #[test]
    fn detail_view_shows_tree_and_demo_name() {
        let graph = Graph {
            title: Some("test".to_string()),
            metric_label: None,
            nodes: vec![GraphNode {
                id: "root".to_string(),
                label: "root-node".to_string(),
                parent: None,
                metric: None,
                metadata: Map::new(),
                ..Default::default()
            }],
        };
        // No real tty in a test process, so `format_tree()`'s own color
        // gating already renders plain text here — same code path detail
        // entry uses, just without a color-coded fixture (see the
        // ansi-parsing test below for that half).
        let text = spex_graph::format_tree(&graph);

        let mut app = App::new(vec![demo("my-demo", "a title", 1, true)]);
        app.mode = Mode::Detail { text: text.clone(), scroll: 0 };
        let mut terminal = Terminal::new(TestBackend::new(90, 10)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let rendered = buffer_text(&terminal);
        assert!(rendered.contains("my-demo"), "{rendered}");
        assert!(rendered.contains("root-node"), "{rendered}");
        assert!(rendered.contains("back"), "{rendered}"); // footer help
    }

    #[test]
    fn port_for_is_deterministic_and_in_range() {
        let a = port_for("decix-trace");
        let b = port_for("decix-trace");
        assert_eq!(a, b);
        assert!((8080..8100).contains(&a));
    }

    #[test]
    fn detail_view_renders_real_ansi_colors_not_raw_escape_codes() {
        let ansi_text = "\u{1b}[38;2;40;110;255mfritz.box\u{1b}[0m  [4.98]";
        let mut app = App::new(vec![demo("decix-trace", "traceroute", 1, true)]);
        app.mode = Mode::Detail { text: ansi_text.to_string(), scroll: 0 };
        let mut terminal = Terminal::new(TestBackend::new(90, 10)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let rendered = buffer_text(&terminal);
        assert!(rendered.contains("fritz.box"), "{rendered}");
        assert!(!rendered.contains('\u{1b}'), "raw escape bytes should never reach the rendered buffer: {rendered:?}");

        // The real proof this is styled, not just stripped-to-plain-text:
        // the "f" of "fritz.box" (one cell inside the block's border) should
        // carry the actual parsed truecolor.
        let buf = terminal.backend().buffer();
        let cell = buf.cell((1, 1)).expect("cell in range");
        assert_eq!(cell.fg, ratatui::style::Color::Rgb(40, 110, 255), "cell: {cell:?}");
    }

    #[test]
    fn filtered_indices_matches_name_or_title_case_insensitively() {
        let demos = vec![
            demo("decix-trace", "traceroute to www.de-cix.net", 10, true),
            demo("sql-schema", "SQL schema: shop.db", 11, true),
            demo("bigmac", "Big Mac Index: United States", 43, true),
        ];
        let mut app = App::new(demos);

        app.filter = "SQL".to_string(); // matches name, case-insensitive
        assert_eq!(app.filtered_indices(), vec![1]);

        app.filter = "index".to_string(); // matches title, case-insensitive
        assert_eq!(app.filtered_indices(), vec![2]);

        app.filter = "nonexistent-xyz".to_string();
        assert!(app.filtered_indices().is_empty());

        app.filter = String::new();
        assert_eq!(app.filtered_indices(), vec![0, 1, 2]);
    }

    #[test]
    fn selected_demo_resolves_through_the_filter() {
        let demos = vec![demo("decix-trace", "traceroute", 10, true), demo("sql-schema", "SQL schema", 11, true)];
        let mut app = App::new(demos);
        app.filter = "sql".to_string();
        app.selected = 0;
        assert_eq!(app.selected_demo().map(|d| d.name.as_str()), Some("sql-schema"));
    }

    #[test]
    fn render_list_shows_match_count_when_filtering() {
        let demos = vec![demo("decix-trace", "traceroute", 10, true), demo("sql-schema", "SQL schema", 11, true)];
        let mut app = App::new(demos);
        app.filter = "sql".to_string();
        let mut terminal = Terminal::new(TestBackend::new(90, 10)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("1/2"), "{text}");
        assert!(text.contains("sql-schema"), "{text}");
        assert!(!text.contains("decix-trace"), "{text}");
    }

    #[test]
    fn render_footer_shows_filter_being_typed() {
        let demos = vec![demo("decix-trace", "traceroute", 10, true)];
        let mut app = App::new(demos);
        app.filtering = true;
        app.filter = "dec".to_string();
        let mut terminal = Terminal::new(TestBackend::new(90, 10)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("/dec"), "{text}");
        assert!(text.contains("done filtering"), "{text}");
    }

    #[test]
    fn animate_view_shows_only_the_current_frame_and_demo_name() {
        let mut app = App::new(vec![demo("my-demo", "a title", 1, true)]);
        app.mode = Mode::Animate {
            frames: vec!["FRAME-ONE".to_string(), "FRAME-TWO".to_string()],
            frame: 1,
            detail_text: "tree text".to_string(),
            detail_scroll: 0,
        };
        let mut terminal = Terminal::new(TestBackend::new(90, 10)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let rendered = buffer_text(&terminal);
        assert!(rendered.contains("my-demo"), "{rendered}");
        assert!(rendered.contains("FRAME-TWO"), "{rendered}");
        assert!(!rendered.contains("FRAME-ONE"), "{rendered}"); // only the current frame index is drawn
        assert!(rendered.contains("stop animation"), "{rendered}"); // footer help
    }

    #[test]
    fn detail_view_footer_advertises_the_animate_key() {
        let mut app = App::new(vec![demo("my-demo", "a title", 1, true)]);
        app.mode = Mode::Detail { text: "tree text".to_string(), scroll: 0 };
        let mut terminal = Terminal::new(TestBackend::new(90, 10)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let rendered = buffer_text(&terminal);
        assert!(rendered.contains("a: animate"), "{rendered}");
    }

    #[test]
    fn build_animation_frames_reports_a_status_message_when_not_web_ready() {
        let d = demo("fresh-capture", "just captured", 3, false);
        let err = build_animation_frames(&d).unwrap_err();
        assert!(err.contains("no tileset yet"), "{err}");
    }

    #[test]
    fn build_animation_frames_produces_a_full_orbit_of_real_frames() {
        // Same code path the 'a' keybinding drives: a real tileset on disk,
        // read back through spex_tiler::read_points and projected through
        // ascii::render_frames, exactly like `spex ascii --animate` does.
        let dir = tempfile::tempdir().unwrap();
        let tileset_dir = dir.path().join("tileset");
        let points = vec![
            spex_core::Point { position: [2.0, 0.0, 0.0], color: [200, 50, 50] },
            spex_core::Point { position: [-2.0, 1.0, 0.5], color: [50, 200, 50] },
        ];
        spex_tiler::build(points, &tileset_dir, &spex_tiler::TilerConfig::default()).unwrap();

        let d = DemoEntry {
            name: "real-tileset".to_string(),
            title: None,
            kind: DemoKind::PointCloud,
            graph_path: None,
            tileset_dir,
            node_count: 2,
            web_ready: true,
        };

        let frames = build_animation_frames(&d).unwrap();
        assert_eq!(frames.len(), ANIMATION_FRAME_COUNT);
        assert!(frames.iter().all(|f| !f.is_empty()), "every frame should render real content");
        // The camera really orbits between frames, so they shouldn't all be
        // byte-identical (mirrors ascii.rs's own render_frames_* tests).
        assert!(frames.windows(2).any(|w| w[0] != w[1]), "frames should differ as the camera orbits");
    }

    #[test]
    fn build_animation_frames_errors_when_tileset_dir_is_missing() {
        let d = DemoEntry {
            name: "phantom".to_string(),
            title: None,
            kind: DemoKind::PointCloud,
            graph_path: None,
            tileset_dir: Path::new("/tmp/definitely-does-not-exist-spex-nav-test").to_path_buf(),
            node_count: 0,
            web_ready: true,
        };
        assert!(build_animation_frames(&d).is_err());
    }
}
