use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
pub const SESSION_INDEX_FILE: &str = "session-index.log";
pub const LAST_SESSION_FILE: &str = "last-session.txt";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIndexEntry {
    pub session_id: String,
    pub profile_name: String,
    pub file_name: String,
}

impl SessionIndexEntry {
    pub fn from_manifest(manifest: &SessionManifest, path: &Path) -> Self {
        let file_name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        Self {
            session_id: manifest.session_id.clone(),
            profile_name: manifest.profile_name.clone(),
            file_name,
        }
    }

    pub fn to_line(&self) -> String {
        format!(
            "{}|{}|{}",
            self.session_id, self.profile_name, self.file_name
        )
    }

    pub fn from_line(line: &str) -> Option<Self> {
        let mut parts = line.split('|');
        let session_id = parts.next()?.to_string();
        let profile_name = parts.next()?.to_string();
        let file_name = parts.next()?.to_string();

        if parts.next().is_some() {
            return None;
        }

        Some(Self {
            session_id,
            profile_name,
            file_name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFileSummary {
    pub session_id: String,
    pub profile_name: String,
    pub event_count: usize,
}

pub fn append_session_index(root: &Path, manifest: &SessionManifest, path: &Path) -> io::Result<()> {
    fs::create_dir_all(root)?;
    let index_path = root.join(SESSION_INDEX_FILE);
    let entry = SessionIndexEntry::from_manifest(manifest, path);
    let mut existing = fs::read_to_string(&index_path).unwrap_or_default();

    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&entry.to_line());
    existing.push('\n');

    fs::write(index_path, existing)
}

pub fn write_last_session_pointer(root: &Path, path: &Path) -> io::Result<()> {
    fs::create_dir_all(root)?;
    fs::write(root.join(LAST_SESSION_FILE), path.display().to_string())
}

pub fn read_last_session_pointer(root: &Path) -> io::Result<Option<PathBuf>> {
    let path = root.join(LAST_SESSION_FILE);
    match fs::read_to_string(path) {
        Ok(contents) => {
            let value = contents.trim();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(PathBuf::from(value)))
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn read_session_index(root: &Path) -> io::Result<Vec<SessionIndexEntry>> {
    let path = root.join(SESSION_INDEX_FILE);
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents
            .lines()
            .filter_map(SessionIndexEntry::from_line)
            .collect()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

pub fn read_session_file_summary(path: &Path) -> io::Result<SessionFileSummary> {
    let contents = fs::read_to_string(path)?;
    let mut session_id = String::new();
    let mut profile_name = String::new();
    let mut event_count = 0usize;

    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("session_id=") {
            session_id = value.to_string();
            continue;
        }

        if let Some(value) = line.strip_prefix("profile=") {
            profile_name = value.to_string();
            continue;
        }

        if !line.trim().is_empty() {
            event_count += 1;
        }
    }

    Ok(SessionFileSummary {
        session_id,
        profile_name,
        event_count,
    })
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
    use super::{
        append_session_index, read_last_session_pointer, read_session_index,
        read_session_file_summary, write_last_session_pointer, SessionManifest, TimelineEvent,
        TimelineEventKind, LAST_SESSION_FILE, SESSION_INDEX_FILE,
    };
    use std::fs;
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

    #[test]
    fn session_index_round_trips_entries() {
        let root = Path::new("data/test-session-index");
        let _ = fs::remove_file(root.join(SESSION_INDEX_FILE));
        let session = SessionManifest::with_profile("privacy");
        let path = session.default_file_path(root);

        append_session_index(root, &session, &path).expect("index append should succeed");
        let entries = read_session_index(root).expect("index read should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].profile_name, "privacy");
        assert!(entries[0].file_name.contains("privacy-session-"));

        let _ = fs::remove_file(root.join(SESSION_INDEX_FILE));
    }

    #[test]
    fn last_session_pointer_round_trips() {
        let root = Path::new("data/test-last-session");
        let _ = fs::remove_file(root.join(LAST_SESSION_FILE));
        let path = root.join("privacy-session-000001.log");

        write_last_session_pointer(root, &path).expect("pointer write should succeed");
        let loaded = read_last_session_pointer(root).expect("pointer read should succeed");

        assert_eq!(loaded, Some(path.clone()));

        let _ = fs::remove_file(root.join(LAST_SESSION_FILE));
    }

    #[test]
    fn session_file_summary_reads_metadata_and_events() {
        let root = Path::new("data/test-session-summary");
        let session = SessionManifest::with_profile("privacy");
        let path = session.write_to_root(root).expect("session write should succeed");
        let summary = read_session_file_summary(&path).expect("summary read should succeed");

        assert!(summary.session_id.starts_with("session-"));
        assert_eq!(summary.profile_name, "privacy");
        assert_eq!(summary.event_count, 0);
    }
}
