use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::data::{
    process::PiInstance,
    session::{SessionEntry, SessionStats},
    sysinfo::SystemStats,
    watcher::WatchEvent,
};

pub type SharedAppState = Arc<Mutex<AppState>>;

#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub session_stats: SessionStats,
    pub system_stats: SystemStats,
    pub current_session_path: Option<PathBuf>,
    pub pi_instances: Vec<PiInstance>,
    pub selected_instance_index: usize,
    pub logs: VecDeque<AppLogEntry>,
    pub max_logs: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session_stats: SessionStats::default(),
            system_stats: SystemStats::default(),
            current_session_path: None,
            pi_instances: Vec::new(),
            selected_instance_index: 0,
            logs: VecDeque::new(),
            max_logs: 500,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_shared() -> SharedAppState {
        Arc::new(Mutex::new(Self::new()))
    }

    pub fn apply_entry(&mut self, entry: &SessionEntry) {
        self.session_stats.apply_entry(entry);
        self.push_log(AppLogEntry::from_entry(entry));
    }

    pub fn replace_session_entries(&mut self, path: PathBuf, entries: &[SessionEntry]) {
        self.session_stats = SessionStats::default();
        self.logs.clear();
        self.current_session_path = Some(path);

        for entry in entries {
            self.apply_entry(entry);
        }
    }

    pub fn apply_watch_event(&mut self, event: WatchEvent) {
        match event {
            WatchEvent::Entry { path, entry } => {
                self.current_session_path = Some(path);
                self.apply_entry(&entry);
            }
            WatchEvent::Error { path, message } => {
                self.current_session_path = path;
                self.push_log(AppLogEntry {
                    timestamp: None,
                    message: format!("watch error: {message}"),
                });
            }
        }
    }

    pub fn apply_system_stats(&mut self, stats: SystemStats) {
        self.system_stats = stats;
    }

    pub fn apply_pi_instances(&mut self, instances: Vec<PiInstance>) {
        self.pi_instances = instances;
        self.clamp_selected_instance();
    }

    pub fn select_next_instance(&mut self) {
        let count = self.associated_instance_count();
        if count == 0 {
            self.selected_instance_index = 0;
            return;
        }
        self.selected_instance_index = (self.selected_instance_index + 1) % count;
    }

    pub fn select_previous_instance(&mut self) {
        let count = self.associated_instance_count();
        if count == 0 {
            self.selected_instance_index = 0;
            return;
        }
        self.selected_instance_index = if self.selected_instance_index == 0 {
            count - 1
        } else {
            self.selected_instance_index - 1
        };
    }

    pub fn selected_instance_session_path(&self) -> Option<PathBuf> {
        self.pi_instances
            .iter()
            .filter(|instance| instance.session_path.is_some() || instance.stats.is_some())
            .nth(self.selected_instance_index)
            .and_then(|instance| instance.session_path.clone())
    }

    pub fn associated_instance_count(&self) -> usize {
        self.pi_instances
            .iter()
            .filter(|instance| instance.session_path.is_some() || instance.stats.is_some())
            .count()
    }

    fn clamp_selected_instance(&mut self) {
        let count = self.associated_instance_count();
        if count == 0 {
            self.selected_instance_index = 0;
        } else {
            self.selected_instance_index = self.selected_instance_index.min(count - 1);
        }
    }

    fn push_log(&mut self, log: AppLogEntry) {
        self.logs.push_back(log);

        while self.logs.len() > self.max_logs {
            self.logs.pop_front();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppLogEntry {
    pub timestamp: Option<String>,
    pub message: String,
}

impl AppLogEntry {
    fn from_entry(entry: &SessionEntry) -> Self {
        match entry {
            SessionEntry::Session(entry) => Self {
                timestamp: Some(entry.timestamp.clone()),
                message: format!("session started: {}", entry.id),
            },
            SessionEntry::Message(entry) => Self {
                timestamp: Some(entry.timestamp.clone()),
                message: format!("{} message", entry.message.role),
            },
            SessionEntry::ModelChange(entry) => Self {
                timestamp: Some(entry.timestamp.clone()),
                message: format!("model: {}/{}", entry.provider, entry.model_id),
            },
            SessionEntry::ThinkingLevelChange(entry) => Self {
                timestamp: Some(entry.timestamp.clone()),
                message: format!("thinking: {}", entry.thinking_level),
            },
            SessionEntry::Compaction(entry) => Self {
                timestamp: Some(entry.timestamp.clone()),
                message: "compaction".to_owned(),
            },
            SessionEntry::ToolExecution(entry) => Self {
                timestamp: entry.timestamp.clone(),
                message: format!("tool: {}", entry.tool_name.as_deref().unwrap_or("unknown")),
            },
            SessionEntry::Other => Self {
                timestamp: None,
                message: "other entry".to_owned(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::data::{process::PiInstance, session::parse_entries};

    use super::*;

    #[test]
    fn apply_entry_updates_session_stats_and_logs() {
        let entries = parse_entries(
            r#"
{"type":"session","version":3,"id":"s1","timestamp":"2026-06-24T00:00:00.000Z","cwd":"/tmp/project"}
{"type":"model_change","id":"m1","parentId":null,"timestamp":"2026-06-24T00:00:01.000Z","provider":"openai-codex","modelId":"gpt-5.5"}
{"type":"message","id":"a1","parentId":"m1","timestamp":"2026-06-24T00:00:02.000Z","message":{"role":"assistant","content":[{"type":"toolCall","name":"bash"}],"usage":{"input":10,"output":20,"totalTokens":30,"cost":{"total":0.25}}}}
"#,
        )
        .expect("valid entries");

        let mut app = AppState::new();
        for entry in &entries {
            app.apply_entry(entry);
        }

        assert_eq!(app.session_stats.session_id.as_deref(), Some("s1"));
        assert_eq!(app.session_stats.current_model.as_deref(), Some("gpt-5.5"));
        assert_eq!(app.session_stats.tokens.total_tokens, 30);
        assert_eq!(app.session_stats.tool_counts.get("bash"), Some(&1));
        assert_eq!(app.logs.len(), 3);
    }

    #[test]
    fn apply_watch_event_sets_current_session_path() {
        let mut app = AppState::new();
        let path = PathBuf::from("session.jsonl");
        let entry = parse_entries(
            r#"{"type":"session","version":3,"id":"s1","timestamp":"t1","cwd":"/tmp"}"#,
        )
        .expect("valid entry")
        .remove(0);

        app.apply_watch_event(WatchEvent::Entry {
            path: path.clone(),
            entry,
        });

        assert_eq!(app.current_session_path, Some(path));
        assert_eq!(app.session_stats.session_id.as_deref(), Some("s1"));
    }

    #[test]
    fn apply_system_stats_replaces_latest_snapshot() {
        let mut app = AppState::new();
        let stats = SystemStats {
            cpu_usage_percent: 12.5,
            memory_used_bytes: 10,
            memory_total_bytes: 20,
            disk_used_bytes: 30,
            disk_total_bytes: 40,
        };

        app.apply_system_stats(stats.clone());

        assert_eq!(app.system_stats, stats);
    }

    #[test]
    fn replace_session_entries_resets_previous_session_state() {
        let first = parse_entries(
            r#"{"type":"session","version":3,"id":"old","timestamp":"t1","cwd":"/tmp"}"#,
        )
        .expect("valid first session");
        let second = parse_entries(
            r#"{"type":"session","version":3,"id":"new","timestamp":"t2","cwd":"/tmp"}"#,
        )
        .expect("valid second session");

        let mut app = AppState::new();
        app.replace_session_entries(PathBuf::from("old.jsonl"), &first);
        app.replace_session_entries(PathBuf::from("new.jsonl"), &second);

        assert_eq!(app.current_session_path, Some(PathBuf::from("new.jsonl")));
        assert_eq!(app.session_stats.session_id.as_deref(), Some("new"));
        assert_eq!(app.logs.len(), 1);
    }

    #[test]
    fn instance_selection_wraps_and_defaults_to_first() {
        let mut app = AppState::new();
        app.apply_pi_instances(vec![
            PiInstance {
                pid: 1,
                ppid: 0,
                memory_bytes: 0,
                cpu_percent: 0.0,
                command: "pi".to_owned(),
                session_path: Some(PathBuf::from("one.jsonl")),
                stats: None,
            },
            PiInstance {
                pid: 2,
                ppid: 0,
                memory_bytes: 0,
                cpu_percent: 0.0,
                command: "pi".to_owned(),
                session_path: Some(PathBuf::from("two.jsonl")),
                stats: None,
            },
        ]);

        assert_eq!(app.selected_instance_index, 0);
        assert_eq!(
            app.selected_instance_session_path(),
            Some(PathBuf::from("one.jsonl"))
        );

        app.select_next_instance();
        assert_eq!(app.selected_instance_index, 1);
        assert_eq!(
            app.selected_instance_session_path(),
            Some(PathBuf::from("two.jsonl"))
        );

        app.select_next_instance();
        assert_eq!(app.selected_instance_index, 0);
    }

    #[test]
    fn logs_are_capped() {
        let mut app = AppState {
            max_logs: 2,
            ..AppState::default()
        };

        for index in 0..3 {
            app.push_log(AppLogEntry {
                timestamp: None,
                message: format!("log {index}"),
            });
        }

        assert_eq!(app.logs.len(), 2);
        assert_eq!(app.logs.front().unwrap().message, "log 1");
        assert_eq!(app.logs.back().unwrap().message, "log 2");
    }
}
