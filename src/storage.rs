use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineStoragePlan {
    pub use_compression: bool,
    pub use_delta_encoding: bool,
}

impl Default for TimelineStoragePlan {
    fn default() -> Self {
        Self {
            use_compression: true,
            use_delta_encoding: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineEventKind {
    RecordingStarted,
    RecordingStopped,
    FrameCaptured,
    MouseActivity,
    KeyboardMetadata,
    ProcessFocusChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvent {
    pub kind: TimelineEventKind,
    pub timestamp_ms: u64,
}

impl TimelineEvent {
    pub fn new(kind: TimelineEventKind) -> Self {
        Self {
            kind,
            timestamp_ms: 0,
        }
    }

    pub fn new_at(kind: TimelineEventKind, timestamp_ms: u64) -> Self {
        Self { kind, timestamp_ms }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionManifest {
    pub session_id: String,
    pub profile_name: String,
    pub events: Vec<TimelineEvent>,
}

impl SessionManifest {
    pub fn new() -> Self {
        Self {
            session_id: next_session_id(),
            profile_name: "privacy".to_string(),
            events: Vec::new(),
        }
    }

    pub fn with_profile(profile_name: impl Into<String>) -> Self {
        Self {
            session_id: next_session_id(),
            profile_name: profile_name.into(),
            events: Vec::new(),
        }
    }

    pub fn push_event(&mut self, event: TimelineEvent) {
        self.events.push(event);
    }

    pub fn to_log_lines(&self) -> Vec<String> {
        self.events
            .iter()
            .map(|event| format!("{}|{}", event.timestamp_ms, event.kind.as_str()))
            .collect()
    }

    pub fn to_file_contents(&self) -> String {
        let mut lines = vec![
            format!("session_id={}", self.session_id),
            format!("profile={}", self.profile_name),
        ];
        lines.extend(self.to_log_lines());
        lines.join("\n")
    }

    pub fn default_file_path(&self, root: &Path) -> PathBuf {
        root.join(format!("{}-{}.log", self.profile_name, self.session_id))
    }

    pub fn write_to_root(&self, root: &Path) -> io::Result<PathBuf> {
        fs::create_dir_all(root)?;
        let path = self.default_file_path(root);
        fs::write(&path, self.to_file_contents())?;
        Ok(path)
    }
}

impl TimelineEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimelineEventKind::RecordingStarted => "recording_started",
            TimelineEventKind::RecordingStopped => "recording_stopped",
            TimelineEventKind::FrameCaptured => "frame_captured",
            TimelineEventKind::MouseActivity => "mouse_activity",
            TimelineEventKind::KeyboardMetadata => "keyboard_metadata",
            TimelineEventKind::ProcessFocusChanged => "process_focus_changed",
        }
    }
}

fn next_session_id() -> String {
    let value = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("session-{value:06}")
}

#[cfg(test)]
mod tests {
    use super::{SessionManifest, TimelineEvent, TimelineEventKind};
    use std::path::Path;

    #[test]
    fn session_manifest_appends_events() {
        let mut session = SessionManifest::new();
        session.push_event(TimelineEvent::new(TimelineEventKind::RecordingStarted));
        assert_eq!(session.events.len(), 1);
    }

    #[test]
    fn session_manifest_serializes_to_log_lines() {
        let mut session = SessionManifest::new();
        session.push_event(TimelineEvent::new(TimelineEventKind::RecordingStarted));
        assert_eq!(session.to_log_lines(), vec!["0|recording_started"]);
    }

    #[test]
    fn session_manifest_builds_default_file_path() {
        let session = SessionManifest::with_profile("focus");
        let path = session.default_file_path(Path::new("data/sessions"));
        assert!(path.to_string_lossy().contains("focus-session-"));
        assert!(path.to_string_lossy().ends_with(".log"));
    }

    #[test]
    fn session_manifest_contents_include_session_metadata() {
        let session = SessionManifest::with_profile("privacy");
        let contents = session.to_file_contents();
        assert!(contents.contains("session_id=session-"));
        assert!(contents.contains("profile=privacy"));
    }
}
