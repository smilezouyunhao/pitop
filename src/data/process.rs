use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use anyhow::{Context, Result};

use super::session::{SessionStats, read_session_file, stats_from_entries};

#[derive(Debug, Clone, PartialEq)]
pub struct PiInstance {
    pub pid: u32,
    pub ppid: u32,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub command: String,
    pub status: String,
    pub session_path: Option<PathBuf>,
    pub stats: Option<SessionStats>,
}

#[derive(Debug, Clone, PartialEq)]
struct ProcessRow {
    pid: u32,
    ppid: u32,
    rss_kb: u64,
    cpu_percent: f32,
    command: String,
}

pub fn discover_pi_instances(sessions_dir: &Path) -> Result<Vec<PiInstance>> {
    let ps_output = Command::new("ps")
        .args(["-ww", "-eo", "pid,ppid,rss,%cpu,command"])
        .output()
        .context("failed to run ps")?;

    if !ps_output.status.success() {
        anyhow::bail!("ps exited with status {}", ps_output.status);
    }

    let text = String::from_utf8_lossy(&ps_output.stdout);
    let current_pid = std::process::id();
    let mut instances = Vec::new();

    for process in parse_ps_output(&text) {
        if process.pid == current_pid || !is_agent_process(&process.command) {
            continue;
        }

        let session_path = session_path_for_pid(process.pid, sessions_dir)
            .ok()
            .flatten();
        instances.push(instance_from_process(process, session_path));
    }

    attach_recent_sessions_to_unmatched_instances(&mut instances, sessions_dir)?;
    for instance in &mut instances {
        instance.status = status_for_instance(instance);
    }
    instances.retain(|instance| instance.session_path.is_some() || instance.stats.is_some());
    instances.sort_by(|left, right| left.pid.cmp(&right.pid));
    Ok(instances)
}

fn instance_from_process(process: ProcessRow, session_path: Option<PathBuf>) -> PiInstance {
    let stats = session_path
        .as_ref()
        .and_then(|path| read_session_file(path).ok())
        .map(|entries| stats_from_entries(&entries));

    PiInstance {
        pid: process.pid,
        ppid: process.ppid,
        memory_bytes: process.rss_kb.saturating_mul(1024),
        cpu_percent: process.cpu_percent,
        command: process.command,
        status: "Unknown".to_owned(),
        session_path,
        stats,
    }
}

fn attach_recent_sessions_to_unmatched_instances(
    instances: &mut [PiInstance],
    sessions_dir: &Path,
) -> Result<()> {
    instances.sort_by(|left, right| left.pid.cmp(&right.pid));

    let used: HashSet<PathBuf> = instances
        .iter()
        .filter_map(|instance| instance.session_path.clone())
        .collect();
    let unmatched_count = instances
        .iter()
        .filter(|instance| instance.session_path.is_none() && is_pi_process(&instance.command))
        .count();
    let mut recent_sessions: Vec<PathBuf> = recent_session_files(sessions_dir)?
        .into_iter()
        .filter(|path| !used.contains(path))
        .take(unmatched_count)
        .collect();
    recent_sessions.reverse();
    let mut recent_sessions = recent_sessions.into_iter();

    for instance in instances
        .iter_mut()
        .filter(|instance| instance.session_path.is_none() && is_pi_process(&instance.command))
    {
        let Some(path) = recent_sessions.next() else {
            break;
        };
        instance.stats = read_session_file(&path)
            .ok()
            .map(|entries| stats_from_entries(&entries));
        instance.session_path = Some(path);
    }

    Ok(())
}

fn recent_session_files(sessions_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_recent_session_files(sessions_dir, &mut files)?;
    files.sort_by(|left, right| right.0.cmp(&left.0));
    Ok(files.into_iter().map(|(_, path)| path).collect())
}

fn collect_recent_session_files(path: &Path, files: &mut Vec<(SystemTime, PathBuf)>) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_file() {
        if is_jsonl(path) {
            let modified = fs::metadata(path)
                .with_context(|| format!("failed to stat session file {}", path.display()))?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            files.push((modified, path.to_path_buf()));
        }
        return Ok(());
    }

    for entry in fs::read_dir(path)
        .with_context(|| format!("failed to read session directory {}", path.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
        collect_recent_session_files(&entry.path(), files)?;
    }

    Ok(())
}

fn status_for_instance(instance: &PiInstance) -> String {
    let Some(stats) = &instance.stats else {
        return "Unknown".to_owned();
    };

    if stats.latest_message_has_tool_call {
        "Executing".to_owned()
    } else if stats.awaiting_assistant {
        "Thinking".to_owned()
    } else {
        "Waiting".to_owned()
    }
}

fn session_path_for_pid(pid: u32, sessions_dir: &Path) -> Result<Option<PathBuf>> {
    let output = Command::new("lsof")
        .args(["-Fn", "-p", &pid.to_string()])
        .output()
        .with_context(|| format!("failed to run lsof for pid {pid}"))?;

    if !output.status.success() {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_lsof_session_path(&text, sessions_dir))
}

fn parse_lsof_session_path(text: &str, sessions_dir: &Path) -> Option<PathBuf> {
    text.lines()
        .filter_map(|line| line.strip_prefix('n'))
        .map(PathBuf::from)
        .find(|path| is_session_jsonl(path, sessions_dir))
}

fn is_session_jsonl(path: &Path, sessions_dir: &Path) -> bool {
    path.starts_with(sessions_dir) && is_jsonl(path)
}

fn is_jsonl(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "jsonl")
}

fn parse_ps_output(text: &str) -> Vec<ProcessRow> {
    text.lines().skip(1).filter_map(parse_ps_line).collect()
}

fn parse_ps_line(line: &str) -> Option<ProcessRow> {
    let mut parts = line.split_whitespace();
    let pid = parts.next()?.parse().ok()?;
    let ppid = parts.next()?.parse().ok()?;
    let rss_kb = parts.next()?.parse().ok()?;
    let cpu_percent = parts.next()?.parse().ok()?;
    let command = parts.collect::<Vec<_>>().join(" ");

    if command.is_empty() {
        return None;
    }

    Some(ProcessRow {
        pid,
        ppid,
        rss_kb,
        cpu_percent,
        command,
    })
}

fn is_agent_process(command: &str) -> bool {
    let command = command.to_ascii_lowercase();

    if command.contains("pitop")
        || command.contains(" ps -ww ")
        || command.contains(" rg ")
        || command.contains("/applications/codex.app/")
        || command.contains("/applications/claude.app/")
        || command.contains("codexbar")
    {
        return false;
    }

    is_pi_process(&command)
}

fn is_pi_process(command: &str) -> bool {
    let command = command.to_ascii_lowercase();
    command.contains("pi-coding-agent")
        || command.contains("@earendil-works/pi-coding-agent")
        || command.contains("/.pi/agent/")
        || contains_command_token(&command, "pi")
}

fn contains_command_token(command: &str, token: &str) -> bool {
    command
        .split(|ch: char| ch.is_whitespace() || ch == '/' || ch == '\\')
        .any(|part| part == token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ps_rows_with_full_command() {
        let text = "  PID  PPID   RSS %CPU COMMAND\n 1234     1 20480  3.5 node /opt/pi-coding-agent/dist/cli.js --flag value\n";
        let rows = parse_ps_output(text);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pid, 1234);
        assert_eq!(rows[0].ppid, 1);
        assert_eq!(rows[0].rss_kb, 20_480);
        assert_eq!(rows[0].cpu_percent, 3.5);
        assert_eq!(
            rows[0].command,
            "node /opt/pi-coding-agent/dist/cli.js --flag value"
        );
    }

    #[test]
    fn detects_agent_processes() {
        assert!(is_agent_process("pi "));
        assert!(is_agent_process(
            "node /opt/@earendil-works/pi-coding-agent/dist/cli.js"
        ));
        assert!(!is_agent_process("/usr/local/bin/codex"));
        assert!(!is_agent_process("/usr/local/bin/claude"));
        assert!(!is_agent_process(
            "/Applications/Codex.app/Contents/MacOS/Codex"
        ));
        assert!(!is_agent_process(
            "/Users/me/Documents/Projects/pitop/target/release/pitop"
        ));
    }

    #[test]
    fn extracts_session_path_from_lsof_name_records() {
        let sessions_dir = Path::new("/Users/me/.pi/agent/sessions");
        let text = "p1234\nn/Users/me/.pi/agent/sessions/project/session.jsonl\nn/tmp/other.txt\n";

        assert_eq!(
            parse_lsof_session_path(text, sessions_dir),
            Some(PathBuf::from(
                "/Users/me/.pi/agent/sessions/project/session.jsonl"
            ))
        );
    }
}
