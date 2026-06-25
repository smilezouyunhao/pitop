use std::{
    env, fs,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use pitop::{
    app::{AppState, SessionChoice},
    data::{
        session::{SessionEntry, parse_entry, read_session_file},
        sysinfo::spawn_system_monitor,
        watcher::{WatchEvent, spawn_session_watcher},
    },
    ui::render_dashboard,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let mut terminal = init_terminal()?;
    let result = run(&mut terminal);
    restore_terminal(&mut terminal)?;
    result
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = AppState::new();

    let cwd = env::current_dir().context("failed to get current working directory")?;
    let sessions_dir = default_sessions_dir();
    load_latest_session_for_cwd(&mut app, &sessions_dir, &cwd)?;

    let (watch_sender, mut watch_receiver) = mpsc::unbounded_channel();
    let _session_watcher = if sessions_dir.exists() {
        match spawn_session_watcher(&sessions_dir, watch_sender) {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                app.apply_watch_event(WatchEvent::Error {
                    path: Some(sessions_dir.clone()),
                    message: error.to_string(),
                });
                None
            }
        }
    } else {
        app.apply_watch_event(WatchEvent::Error {
            path: Some(sessions_dir.clone()),
            message: "sessions directory not found".to_owned(),
        });
        None
    };

    let (mut system_receiver, system_task) = spawn_system_monitor(Duration::from_secs(1));
    app.apply_system_stats(system_receiver.borrow().clone());

    loop {
        drain_watch_events(&mut app, &mut watch_receiver, &cwd);
        if system_receiver.has_changed().unwrap_or(false) {
            app.apply_system_stats(system_receiver.borrow_and_update().clone());
        }

        terminal.draw(|frame| render_dashboard(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if handle_key_event(&mut app, key, &sessions_dir)? {
                    break;
                }
            }
        }
    }

    system_task.abort();
    Ok(())
}

fn drain_watch_events(
    app: &mut AppState,
    receiver: &mut mpsc::UnboundedReceiver<WatchEvent>,
    cwd: &Path,
) {
    while let Ok(event) = receiver.try_recv() {
        match event {
            WatchEvent::Entry { path, entry } => {
                let should_load_file = app.current_session_path.as_ref() != Some(&path)
                    || app.session_stats.session_id.is_none();

                if should_load_file {
                    match load_session_file_if_matches_cwd(app, &path, cwd) {
                        Ok(true) => {}
                        Ok(false) => {}
                        Err(error) => app.apply_watch_event(WatchEvent::Error {
                            path: Some(path),
                            message: error.to_string(),
                        }),
                    }
                } else {
                    app.apply_entry(&entry);
                }
            }
            WatchEvent::Error { path, message } => {
                app.apply_watch_event(WatchEvent::Error { path, message });
            }
        }
    }
}

fn handle_key_event(app: &mut AppState, key: KeyEvent, sessions_dir: &Path) -> Result<bool> {
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

    if app.session_picker_open {
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Esc => app.close_session_picker(),
            KeyCode::Up => app.select_previous_session(),
            KeyCode::Down => app.select_next_session(),
            KeyCode::Enter => {
                if let Some(path) = app.selected_session_path() {
                    load_session_file(app, &path)?;
                    app.close_session_picker();
                }
            }
            _ => {}
        }

        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Ok(true),
        KeyCode::Char('s') => {
            let choices = collect_session_choices(sessions_dir)?;
            app.open_session_picker(choices);
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn default_sessions_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent")
        .join("sessions")
}

fn load_latest_session_for_cwd(app: &mut AppState, sessions_dir: &Path, cwd: &Path) -> Result<()> {
    let session_file = match find_latest_session_file(sessions_dir, cwd)? {
        Some(path) => Some(path),
        None => find_latest_any_session_file(sessions_dir)?,
    };

    let Some(session_file) = session_file else {
        return Ok(());
    };

    load_session_file(app, &session_file)
}

fn load_session_file_if_matches_cwd(app: &mut AppState, path: &Path, cwd: &Path) -> Result<bool> {
    let cwd = cwd.to_string_lossy();
    if !session_file_matches_cwd(path, &cwd)? {
        return Ok(false);
    }

    load_session_file(app, path)?;
    Ok(true)
}

fn load_session_file(app: &mut AppState, path: &Path) -> Result<()> {
    let entries = read_session_file(path)?;
    app.replace_session_entries(path.to_path_buf(), &entries);
    Ok(())
}

fn collect_session_choices(sessions_dir: &Path) -> Result<Vec<SessionChoice>> {
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut choices = Vec::new();
    collect_session_choices_inner(sessions_dir, &mut choices)?;
    choices.sort_by(|left, right| right.0.cmp(&left.0));

    Ok(choices
        .into_iter()
        .take(200)
        .map(|(_, choice)| choice)
        .collect())
}

fn collect_session_choices_inner(
    path: &Path,
    choices: &mut Vec<(SystemTime, SessionChoice)>,
) -> Result<()> {
    if path.is_file() {
        if is_jsonl(path) {
            if let Some(choice) = read_session_choice(path)? {
                let modified = fs::metadata(path)
                    .with_context(|| format!("failed to stat session file {}", path.display()))?
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                choices.push((modified, choice));
            }
        }
        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)
            .with_context(|| format!("failed to read session directory {}", path.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
            collect_session_choices_inner(&entry.path(), choices)?;
        }
    }

    Ok(())
}

fn read_session_choice(path: &Path) -> Result<Option<SessionChoice>> {
    let file = fs::File::open(path)
        .with_context(|| format!("failed to open session file {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .with_context(|| format!("failed to read session header {}", path.display()))?;

    match parse_entry(line.trim())? {
        SessionEntry::Session(header) => Ok(Some(SessionChoice {
            path: path.to_path_buf(),
            id: header.id,
            cwd: header.cwd,
            timestamp: header.timestamp,
        })),
        _ => Ok(None),
    }
}

fn find_latest_session_file(sessions_dir: &Path, cwd: &Path) -> Result<Option<PathBuf>> {
    if !sessions_dir.exists() {
        return Ok(None);
    }

    let cwd = cwd.to_string_lossy();
    let mut latest: Option<(SystemTime, PathBuf)> = None;
    find_latest_session_file_inner(sessions_dir, &cwd, &mut latest)?;

    Ok(latest.map(|(_, path)| path))
}

fn find_latest_any_session_file(sessions_dir: &Path) -> Result<Option<PathBuf>> {
    if !sessions_dir.exists() {
        return Ok(None);
    }

    let mut latest: Option<(SystemTime, PathBuf)> = None;
    find_latest_any_session_file_inner(sessions_dir, &mut latest)?;

    Ok(latest.map(|(_, path)| path))
}

fn find_latest_session_file_inner(
    path: &Path,
    cwd: &str,
    latest: &mut Option<(SystemTime, PathBuf)>,
) -> Result<()> {
    if path.is_file() {
        if is_jsonl(path) && session_file_matches_cwd(path, cwd)? {
            let modified = fs::metadata(path)
                .with_context(|| format!("failed to stat session file {}", path.display()))?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);

            if latest
                .as_ref()
                .is_none_or(|(latest_modified, _)| modified > *latest_modified)
            {
                *latest = Some((modified, path.to_path_buf()));
            }
        }
        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)
            .with_context(|| format!("failed to read session directory {}", path.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
            find_latest_session_file_inner(&entry.path(), cwd, latest)?;
        }
    }

    Ok(())
}

fn find_latest_any_session_file_inner(
    path: &Path,
    latest: &mut Option<(SystemTime, PathBuf)>,
) -> Result<()> {
    if path.is_file() {
        if is_jsonl(path) {
            let modified = fs::metadata(path)
                .with_context(|| format!("failed to stat session file {}", path.display()))?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);

            if latest
                .as_ref()
                .is_none_or(|(latest_modified, _)| modified > *latest_modified)
            {
                *latest = Some((modified, path.to_path_buf()));
            }
        }
        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)
            .with_context(|| format!("failed to read session directory {}", path.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
            find_latest_any_session_file_inner(&entry.path(), latest)?;
        }
    }

    Ok(())
}

fn session_file_matches_cwd(path: &Path, cwd: &str) -> Result<bool> {
    let file = fs::File::open(path)
        .with_context(|| format!("failed to open session file {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .with_context(|| format!("failed to read session header {}", path.display()))?;

    match parse_entry(line.trim())? {
        SessionEntry::Session(header) => Ok(path_matches_cwd(&header.cwd, cwd)),
        _ => Ok(false),
    }
}

fn path_matches_cwd(session_cwd: &str, cwd: &str) -> bool {
    if session_cwd == cwd {
        return true;
    }

    let session_cwd = Path::new(session_cwd).canonicalize().ok();
    let cwd = Path::new(cwd).canonicalize().ok();

    session_cwd.is_some() && session_cwd == cwd
}

fn is_jsonl(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "jsonl")
}
