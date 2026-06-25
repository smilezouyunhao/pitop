use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CostUsage {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionStats {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub started_at: Option<String>,
    pub latest_timestamp: Option<String>,
    pub current_provider: Option<String>,
    pub current_model: Option<String>,
    pub thinking_level: Option<String>,
    pub message_count: u64,
    pub turn_count: u64,
    pub compactions: u64,
    pub latest_context_tokens: Option<u64>,
    pub tokens: TokenUsage,
    pub cost: CostUsage,
    pub tool_counts: BTreeMap<String, u64>,
}

impl SessionStats {
    pub fn apply_entry(&mut self, entry: &SessionEntry) {
        match entry {
            SessionEntry::Session(header) => {
                self.session_id = Some(header.id.clone());
                self.cwd = Some(header.cwd.clone());
                self.started_at = Some(header.timestamp.clone());
                self.latest_timestamp = Some(header.timestamp.clone());
            }
            SessionEntry::Message(entry) => {
                self.message_count += 1;
                if entry.message.role == "user" {
                    self.turn_count += 1;
                }
                self.latest_timestamp = Some(entry.timestamp.clone());

                if let Some(usage) = &entry.message.usage {
                    self.tokens.input += usage.input.unwrap_or_default();
                    self.tokens.output += usage.output.unwrap_or_default();
                    self.tokens.cache_read += usage.cache_read.unwrap_or_default();
                    self.tokens.cache_write += usage.cache_write.unwrap_or_default();
                    if let Some(total_tokens) = usage.total_tokens {
                        self.latest_context_tokens = Some(total_tokens);
                        self.tokens.total_tokens += total_tokens;
                    }

                    if let Some(cost) = &usage.cost {
                        self.cost.input += cost.input.unwrap_or_default();
                        self.cost.output += cost.output.unwrap_or_default();
                        self.cost.cache_read += cost.cache_read.unwrap_or_default();
                        self.cost.cache_write += cost.cache_write.unwrap_or_default();
                        self.cost.total += cost.total.unwrap_or_default();
                    }
                }

                for tool_name in entry.message.tool_call_names() {
                    *self.tool_counts.entry(tool_name.to_owned()).or_default() += 1;
                }
            }
            SessionEntry::ModelChange(entry) => {
                self.latest_timestamp = Some(entry.timestamp.clone());
                self.current_provider = Some(entry.provider.clone());
                self.current_model = Some(entry.model_id.clone());
            }
            SessionEntry::ThinkingLevelChange(entry) => {
                self.latest_timestamp = Some(entry.timestamp.clone());
                self.thinking_level = Some(entry.thinking_level.clone());
            }
            SessionEntry::Compaction(entry) => {
                self.latest_timestamp = Some(entry.timestamp.clone());
                self.compactions += 1;
            }
            SessionEntry::ToolExecution(entry) => {
                self.latest_timestamp = entry.timestamp.clone();
                if let Some(tool_name) = entry.tool_name() {
                    *self.tool_counts.entry(tool_name.to_owned()).or_default() += 1;
                }
            }
            SessionEntry::Other => {}
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SessionEntry {
    #[serde(rename = "session")]
    Session(SessionHeader),
    #[serde(rename = "message")]
    Message(MessageEntry),
    #[serde(rename = "model_change")]
    ModelChange(ModelChangeEntry),
    #[serde(rename = "thinking_level_change")]
    ThinkingLevelChange(ThinkingLevelChangeEntry),
    #[serde(rename = "compaction")]
    Compaction(CompactionEntry),
    #[serde(rename = "tool_execution", alias = "tool")]
    ToolExecution(ToolExecutionEntry),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SessionHeader {
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MessageEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub message: AgentMessage,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ModelChangeEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub provider: String,
    #[serde(rename = "modelId")]
    pub model_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ThinkingLevelChangeEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "thinkingLevel")]
    pub thinking_level: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CompactionEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "tokensBefore")]
    pub tokens_before: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ToolExecutionEntry {
    pub timestamp: Option<String>,
    #[serde(
        rename = "toolName",
        alias = "tool_name",
        alias = "name",
        alias = "tool"
    )]
    pub tool_name: Option<String>,
}

impl ToolExecutionEntry {
    fn tool_name(&self) -> Option<&str> {
        self.tool_name.as_deref().filter(|name| !name.is_empty())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AgentMessage {
    pub role: String,
    pub content: Option<MessageContent>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub usage: Option<MessageUsage>,
}

impl AgentMessage {
    fn tool_call_names(&self) -> Vec<&str> {
        let Some(MessageContent::Blocks(blocks)) = &self.content else {
            return Vec::new();
        };

        blocks
            .iter()
            .filter(|block| block.kind == "toolCall")
            .filter_map(|block| block.name.as_deref())
            .filter(|name| !name.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MessageUsage {
    pub input: Option<u64>,
    pub output: Option<u64>,
    #[serde(rename = "cacheRead")]
    pub cache_read: Option<u64>,
    #[serde(rename = "cacheWrite")]
    pub cache_write: Option<u64>,
    #[serde(rename = "totalTokens")]
    pub total_tokens: Option<u64>,
    pub cost: Option<MessageCost>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MessageCost {
    pub input: Option<f64>,
    pub output: Option<f64>,
    #[serde(rename = "cacheRead")]
    pub cache_read: Option<f64>,
    #[serde(rename = "cacheWrite")]
    pub cache_write: Option<f64>,
    pub total: Option<f64>,
}

pub fn parse_entry(line: &str) -> Result<SessionEntry> {
    serde_json::from_str(line).context("failed to parse session JSONL entry")
}

pub fn parse_entries(content: &str) -> Result<Vec<SessionEntry>> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            parse_entry(line).with_context(|| format!("failed to parse line {}", index + 1))
        })
        .collect()
}

pub fn read_session_file(path: impl AsRef<Path>) -> Result<Vec<SessionEntry>> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read session file {}", path.display()))?;
    parse_entries(&content)
}

pub fn stats_from_entries(entries: &[SessionEntry]) -> SessionStats {
    let mut stats = SessionStats::default();
    for entry in entries {
        stats.apply_entry(entry);
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jsonl_and_aggregates_session_stats() {
        let content = r#"
{"type":"session","version":3,"id":"session-1","timestamp":"2026-06-24T00:00:00.000Z","cwd":"/tmp/project"}
{"type":"model_change","id":"m1","parentId":null,"timestamp":"2026-06-24T00:00:01.000Z","provider":"openai-codex","modelId":"gpt-5.5"}
{"type":"thinking_level_change","id":"t1","parentId":"m1","timestamp":"2026-06-24T00:00:02.000Z","thinkingLevel":"medium"}
{"type":"message","id":"u1","parentId":"t1","timestamp":"2026-06-24T00:00:03.000Z","message":{"role":"user","content":"please check"}}
{"type":"message","id":"a1","parentId":"u1","timestamp":"2026-06-24T00:00:04.000Z","message":{"role":"assistant","content":[{"type":"text","text":"checking"},{"type":"toolCall","name":"bash"},{"type":"toolCall","name":"read"}],"usage":{"input":10,"output":20,"cacheRead":30,"cacheWrite":40,"totalTokens":100,"cost":{"input":0.1,"output":0.2,"cacheRead":0.03,"cacheWrite":0.04,"total":0.37}}}}
{"type":"message","id":"a2","parentId":"a1","timestamp":"2026-06-24T00:00:05.000Z","message":{"role":"assistant","content":[{"type":"toolCall","name":"bash"}],"usage":{"input":1,"output":2,"totalTokens":3,"cost":{"total":0.5}}}}
{"type":"compaction","id":"c1","parentId":"a2","timestamp":"2026-06-24T00:00:05.000Z","summary":"short","firstKeptEntryId":"a2","tokensBefore":1000}
"#;

        let entries = parse_entries(content).expect("valid entries");
        let stats = stats_from_entries(&entries);

        assert_eq!(stats.session_id.as_deref(), Some("session-1"));
        assert_eq!(stats.cwd.as_deref(), Some("/tmp/project"));
        assert_eq!(stats.current_provider.as_deref(), Some("openai-codex"));
        assert_eq!(stats.current_model.as_deref(), Some("gpt-5.5"));
        assert_eq!(stats.thinking_level.as_deref(), Some("medium"));
        assert_eq!(stats.message_count, 3);
        assert_eq!(stats.turn_count, 1);
        assert_eq!(stats.latest_context_tokens, Some(3));
        assert_eq!(stats.compactions, 1);
        assert_eq!(stats.tokens.input, 11);
        assert_eq!(stats.tokens.output, 22);
        assert_eq!(stats.tokens.cache_read, 30);
        assert_eq!(stats.tokens.cache_write, 40);
        assert_eq!(stats.tokens.total_tokens, 103);
        assert!((stats.cost.total - 0.87).abs() < f64::EPSILON);
        assert_eq!(stats.tool_counts.get("bash"), Some(&2));
        assert_eq!(stats.tool_counts.get("read"), Some(&1));
    }

    #[test]
    fn supports_legacy_tool_execution_entries() {
        let entry = parse_entry(
            r#"{"type":"tool_execution","timestamp":"2026-06-24T00:00:00.000Z","toolName":"edit"}"#,
        )
        .expect("valid tool execution entry");

        let stats = stats_from_entries(&[entry]);
        assert_eq!(stats.tool_counts.get("edit"), Some(&1));
    }
}
