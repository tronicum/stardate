//! `spex nav` — a k9s-style interactive browser over `discover_demos()`'s
//! output: move through the list, view a demo's tree inline, or launch its
//! web view, without re-typing a path into a fresh command each time.
use crate::{discover_demos, DemoEntry};
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

enum Mode {
    List,
    Detail { text: String, scroll: u16 },
}

struct App {
    demos: Vec<DemoEntry>,
    selected: usize,
    mode: Mode,
    status: Option<String>,
}

impl App {
    fn new(demos: Vec<DemoEntry>) -> Self {
        App {
            demos,
            selected: 0,
            mode: Mode::List,
            status: None,
        }
    }

    fn selected_demo(&self) -> Option<&DemoEntry> {
        self.demos.get(self.selected)
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

        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if matches!(app.mode, Mode::Detail { .. }) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => app.mode = Mode::List,
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
        } else {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.selected > 0 {
                        app.selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.selected + 1 < app.demos.len() {
                        app.selected += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('v') => {
                    if let Some(demo) = app.selected_demo() {
                        let text = spex_graph::Graph::read_json(&demo.graph_path)
                            .map(|g| strip_ansi(&spex_graph::format_tree(&g)))
                            .unwrap_or_else(|e| format!("failed to read {}: {e}", demo.graph_path.display()));
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
        return format!(
            "{} has no tileset yet — run `spex graph-layout {} -o {}` first",
            demo.name,
            demo.graph_path.display(),
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

/// Strips ANSI color escapes from `format_tree()`'s output — rendering real
/// color inside a ratatui `Paragraph` needs an ANSI-to-styled-text
/// conversion that's out of scope for this pass (see plan notes); plain
/// text is still fully legible.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    match &app.mode {
        Mode::List => render_list(f, chunks[0], app),
        Mode::Detail { text, scroll } => render_detail(f, chunks[0], app, text, *scroll),
    }
    render_footer(f, chunks[1], app);
}

fn render_list(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .demos
        .iter()
        .map(|d| {
            let title = d.title.clone().unwrap_or_else(|| d.name.clone());
            let ready = if d.web_ready { "" } else { "  (no tileset yet)" };
            ListItem::new(format!("{:<16} {:>4} nodes   {title}{ready}", d.name, d.node_count))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected));

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" spex nav — demos "))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(list, area, &mut state);
}

fn render_detail(f: &mut Frame, area: Rect, app: &App, text: &str, scroll: u16) {
    let name = app.selected_demo().map(|d| d.name.as_str()).unwrap_or("");
    let paragraph = Paragraph::new(text.to_string())
        .block(Block::default().borders(Borders::ALL).title(format!(" {name} ")))
        .scroll((scroll, 0));
    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let help = match app.mode {
        Mode::List => "\u{2191}/k \u{2193}/j: move   enter/v: view tree   w: open web view   q: quit",
        Mode::Detail { .. } => "\u{2191}/k \u{2193}/j: scroll   esc/q: back",
    };
    let text = match &app.status {
        Some(s) => format!("{help}   |   {s}"),
        None => help.to_string(),
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
            graph_path: Path::new("/tmp/does-not-matter/graph.json").to_path_buf(),
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
    fn detail_view_shows_stripped_tree_and_demo_name() {
        let graph = Graph {
            title: Some("test".to_string()),
            metric_label: None,
            nodes: vec![GraphNode {
                id: "root".to_string(),
                label: "root-node".to_string(),
                parent: None,
                metric: None,
                metadata: Map::new(),
            }],
        };
        let text = strip_ansi(&spex_graph::format_tree(&graph));
        assert!(!text.contains('\u{1b}'), "should have no raw ANSI escapes left: {text:?}");

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
    fn strip_ansi_removes_color_codes_but_keeps_text() {
        let input = "\u{1b}[38;2;40;110;255mfritz.box\u{1b}[0m  [4.98]";
        assert_eq!(strip_ansi(input), "fritz.box  [4.98]");
    }
}
