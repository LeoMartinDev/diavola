use std::collections::{HashMap, VecDeque};

use regex::RegexBuilder;

use crate::domain::{
    process::LogStream,
    runtime::{ProcessLogPayload, ProcessRuntimeId},
};
use crate::error::AppError;
use crate::infrastructure::ansi::strip_ansi;

#[derive(Debug, Clone)]
pub struct FlatRowEntry {
    pub text: String,
    pub stream: LogStream,
}

#[derive(Debug, Clone, Default)]
pub struct SearchMatches {
    pub match_count: usize,
    pub match_indices: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct InMemoryLogStore {
    limit: usize,
    entries: HashMap<ProcessRuntimeId, VecDeque<FlatRowEntry>>,
}

impl Default for InMemoryLogStore {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl InMemoryLogStore {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            entries: HashMap::new(),
        }
    }

    pub fn append(&mut self, payload: &ProcessLogPayload) {
        let queue = self.entries.entry(payload.runtime_id.clone()).or_default();
        for line in &payload.lines {
            queue.push_back(FlatRowEntry {
                text: line.clone(),
                stream: payload.stream,
            });
        }
        while queue.len() > self.limit {
            queue.pop_front();
        }
    }

    pub fn len(&self, runtime_id: &ProcessRuntimeId) -> usize {
        self.entries.get(runtime_id).map(|q| q.len()).unwrap_or(0)
    }

    pub fn clear(&mut self, runtime_id: &ProcessRuntimeId) {
        if let Some(queue) = self.entries.get_mut(runtime_id) {
            queue.clear();
        }
    }

    pub fn search(
        &self,
        runtime_id: &ProcessRuntimeId,
        query: &str,
        regex: bool,
        case_sensitive: bool,
        up_to: usize,
    ) -> Result<SearchMatches, AppError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(SearchMatches::default());
        }
        let source = if regex {
            trimmed.to_string()
        } else {
            escape_regex(trimmed)
        };
        let pattern = RegexBuilder::new(&source)
            .case_insensitive(!case_sensitive)
            .build()
            .map_err(|e| AppError::runtime(format!("invalid search regex: {e}")))?;

        let Some(queue) = self.entries.get(runtime_id) else {
            return Ok(SearchMatches::default());
        };
        let limit = up_to.min(queue.len());

        let mut indices = Vec::new();
        for (i, entry) in queue.iter().take(limit).enumerate() {
            let target = format!("{} {}", entry.stream.as_str(), strip_ansi(&entry.text));
            if pattern.is_match(&target) {
                indices.push(i as u32);
            }
        }
        Ok(SearchMatches {
            match_count: indices.len(),
            match_indices: indices,
        })
    }
}

fn escape_regex(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        if matches!(
            ch,
            '.' | '*' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::runtime::{ProcessLogPayload, ProcessRuntimeId, RunSessionId};

    fn payload(runtime: &ProcessRuntimeId, stream: LogStream, lines: &[&str]) -> ProcessLogPayload {
        ProcessLogPayload {
            session_id: RunSessionId::new(),
            runtime_id: runtime.clone(),
            process_name: "api".into(),
            stream,
            lines: lines.iter().map(|s| (*s).to_string()).collect(),
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn append_flattens_lines_into_rows() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["a", "b", "c"]));
        assert_eq!(store.len(&runtime), 3);
    }

    #[test]
    fn caps_at_limit_and_drops_from_front() {
        let mut store = InMemoryLogStore::new(3);
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["first", "second", "third", "fourth"]));
        // 4 appended, cap 3 → front ("first") dropped. Remaining: second, third, fourth.
        // Search "second" — "stdout second" contains "second" → index 0.
        let matches = store.search(&runtime, "second", false, false, usize::MAX).unwrap();
        assert_eq!(matches.match_indices, vec![0]);
        // "first" is gone.
        let matches = store.search(&runtime, "first", false, false, usize::MAX).unwrap();
        assert!(matches.match_indices.is_empty());
    }

    #[test]
    fn literal_substring_match_is_case_insensitive_by_default() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["Listening on 3000"]));
        let m = store
            .search(&runtime, "listening", false, false, usize::MAX)
            .unwrap();
        assert_eq!(m.match_indices, vec![0]);
    }

    #[test]
    fn case_sensitive_match_respects_casing() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(
            &runtime,
            LogStream::Stdout,
            &["Listening on 3000", "listening on 3001"],
        ));
        let m = store
            .search(&runtime, "Listening", false, true, usize::MAX)
            .unwrap();
        assert_eq!(m.match_indices, vec![0]);
    }

    #[test]
    fn regex_mode_uses_pattern() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(
            &runtime,
            LogStream::Stdout,
            &["error 42", "warn 7", "error 99"],
        ));
        let m = store
            .search(&runtime, r"error \d+", true, false, usize::MAX)
            .unwrap();
        assert_eq!(m.match_indices, vec![0, 2]);
    }

    #[test]
    fn matches_against_stream_prefixed_target() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stderr, &["boom"]));
        let m = store
            .search(&runtime, "stderr", false, false, usize::MAX)
            .unwrap();
        assert_eq!(m.match_indices, vec![0]);
    }

    #[test]
    fn strips_ansi_before_matching() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(
            &runtime,
            LogStream::Stdout,
            &["\x1b[32mready\x1b[39m"],
        ));
        let m = store.search(&runtime, "ready", false, false, usize::MAX).unwrap();
        assert_eq!(m.match_indices, vec![0]);
    }

    #[test]
    fn up_to_truncates_considered_rows() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["hit", "hit", "hit"]));
        let m = store.search(&runtime, "hit", false, false, 2).unwrap();
        assert_eq!(m.match_indices, vec![0, 1]);
    }

    #[test]
    fn empty_query_returns_no_matches() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["x"]));
        let m = store.search(&runtime, "   ", false, false, usize::MAX).unwrap();
        assert_eq!(m.match_count, 0);
    }

    #[test]
    fn invalid_regex_returns_error() {
        let store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        let result = store.search(&runtime, "(unclosed", true, false, usize::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_runtime_returns_no_matches() {
        let store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        let m = store
            .search(&runtime, "anything", false, false, usize::MAX)
            .unwrap();
        assert_eq!(m.match_count, 0);
    }

    #[test]
    fn clear_removes_rows() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["a"]));
        store.clear(&runtime);
        assert_eq!(store.len(&runtime), 0);
    }
}
