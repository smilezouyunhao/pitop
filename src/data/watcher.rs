use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::mpsc as std_mpsc,
    thread,
};

use anyhow::{Context, Result};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use super::session::{SessionEntry, parse_entry};

#[derive(Debug)]
pub enum WatchEvent {
    Entry {
        path: PathBuf,
        entry: SessionEntry,
    },
    Error {
        path: Option<PathBuf>,
        message: String,
    },
}

pub struct SessionWatcher {
    _watcher: RecommendedWatcher,
    _worker: thread::JoinHandle<()>,
}

pub fn spawn_session_watcher(
    sessions_dir: impl AsRef<Path>,
    sender: mpsc::UnboundedSender<WatchEvent>,
) -> Result<SessionWatcher> {
    let sessions_dir = sessions_dir.as_ref();
    let (notify_tx, notify_rx) = std_mpsc::channel();

    let mut watcher = RecommendedWatcher::new(notify_tx, Config::default())
        .context("failed to create session watcher")?;
    watcher
        .watch(sessions_dir, RecursiveMode::Recursive)
        .with_context(|| format!("failed to watch {}", sessions_dir.display()))?;

    let worker = thread::spawn(move || {
        let mut tails = TailReaders::default();

        while let Ok(event) = notify_rx.recv() {
            match event {
                Ok(event) => handle_notify_event(event, &mut tails, &sender),
                Err(error) => {
                    let _ = sender.send(WatchEvent::Error {
                        path: None,
                        message: error.to_string(),
                    });
                }
            }
        }
    });

    Ok(SessionWatcher {
        _watcher: watcher,
        _worker: worker,
    })
}

fn handle_notify_event(
    event: Event,
    tails: &mut TailReaders,
    sender: &mpsc::UnboundedSender<WatchEvent>,
) {
    for path in event.paths.into_iter().filter(|path| is_jsonl(path)) {
        match tails.read_new_entries(&path) {
            Ok(entries) => {
                for entry in entries {
                    let _ = sender.send(WatchEvent::Entry {
                        path: path.clone(),
                        entry,
                    });
                }
            }
            Err(error) => {
                let _ = sender.send(WatchEvent::Error {
                    path: Some(path),
                    message: error.to_string(),
                });
            }
        }
    }
}

fn is_jsonl(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "jsonl")
}

#[derive(Debug, Default)]
pub struct TailReaders {
    offsets: HashMap<PathBuf, u64>,
    pending: HashMap<PathBuf, String>,
}

impl TailReaders {
    pub fn read_new_entries(&mut self, path: impl AsRef<Path>) -> Result<Vec<SessionEntry>> {
        let path = path.as_ref();
        let mut file = File::open(path)
            .with_context(|| format!("failed to open session file {}", path.display()))?;
        let file_len = file
            .metadata()
            .with_context(|| format!("failed to stat session file {}", path.display()))?
            .len();

        let offset = self.offsets.entry(path.to_path_buf()).or_insert(0);
        if file_len < *offset {
            *offset = 0;
            self.pending.remove(path);
        }

        file.seek(SeekFrom::Start(*offset))
            .with_context(|| format!("failed to seek session file {}", path.display()))?;

        let mut chunk = String::new();
        file.read_to_string(&mut chunk)
            .with_context(|| format!("failed to read session file {}", path.display()))?;
        *offset = file_len;

        if chunk.is_empty() {
            return Ok(Vec::new());
        }

        let mut text = self.pending.remove(path).unwrap_or_default();
        text.push_str(&chunk);

        let has_complete_tail = text.ends_with('\n');
        let mut lines: Vec<&str> = text.lines().collect();
        if !has_complete_tail {
            if let Some(partial) = lines.pop() {
                self.pending.insert(path.to_path_buf(), partial.to_owned());
            }
        }

        lines
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .map(parse_entry)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::OpenOptions, io::Write};

    use super::*;

    #[test]
    fn reads_only_new_complete_jsonl_lines() {
        let path = std::env::temp_dir().join(format!("pitop-watcher-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);

        {
            let mut file = File::create(&path).expect("create temp session");
            writeln!(
                file,
                "{{\"type\":\"session\",\"id\":\"s1\",\"timestamp\":\"t1\",\"cwd\":\"/tmp\"}}"
            )
            .expect("write header");
        }

        let mut tails = TailReaders::default();
        let first = tails.read_new_entries(&path).expect("first read");
        assert_eq!(first.len(), 1);

        {
            let mut file = OpenOptions::new().append(true).open(&path).expect("append");
            write!(file, "{{\"type\":\"model_change\",\"id\":\"m1\",\"parentId\":null,\"timestamp\":\"t2\",\"provider\":\"p\",\"modelId\":\"m\"}}")
                .expect("write partial");
        }

        let second = tails.read_new_entries(&path).expect("partial read");
        assert!(second.is_empty());

        {
            let mut file = OpenOptions::new().append(true).open(&path).expect("append");
            writeln!(file).expect("finish line");
            writeln!(file, "{{\"type\":\"thinking_level_change\",\"id\":\"t1\",\"parentId\":\"m1\",\"timestamp\":\"t3\",\"thinkingLevel\":\"high\"}}")
                .expect("write next line");
        }

        let third = tails.read_new_entries(&path).expect("third read");
        assert_eq!(third.len(), 2);
        assert!(matches!(third[0], SessionEntry::ModelChange(_)));
        assert!(matches!(third[1], SessionEntry::ThinkingLevelChange(_)));

        let fourth = tails.read_new_entries(&path).expect("fourth read");
        assert!(fourth.is_empty());

        let _ = std::fs::remove_file(path);
    }
}
