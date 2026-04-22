use crate::capture::{CaptureBackend, CapturePlan, StubCaptureBackend};
use crate::clock::SessionClock;
use crate::config::{AppConfig, ConfigValidationError, RecordingPlan, RecordingProfile};
use crate::process::{ProcessBackend, ProcessTrackingPlan, StubProcessBackend};
use crate::storage::{
    append_session_index, read_last_session_pointer, read_session_index, write_last_session_pointer,
    read_session_file_summary, SessionFileSummary, SessionIndexEntry, SessionManifest,
    TimelineEvent, TimelineEventKind,
};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecorderState {
    Idle,
    Recording,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecorderStatus {
    pub state: RecorderState,
    pub session_id: String,
    pub profile: RecordingProfile,
    pub profile_summary: &'static str,
    pub screen_capture: bool,
    pub keyboard_events: bool,
    pub mouse_events: bool,
    pub process_tracking: bool,
    pub encryption_enabled: bool,
    pub excluded_app_count: usize,
    pub allowed_app_count: usize,
    pub warnings: Vec<&'static str>,
    pub pending_issue: Option<&'static str>,
    pub session_event_count: usize,
    pub capture_backend: &'static str,
    pub process_backend: &'static str,
    pub session_path: Option<PathBuf>,
    pub indexed_session_count: usize,
    pub last_persisted_session_path: Option<PathBuf>,
    pub last_session_summary: Option<SessionFileSummary>,
}

#[derive(Debug, Clone)]
pub struct Recorder {
    config: AppConfig,
    state: RecorderState,
    session: SessionManifest,
    capture_plan: CapturePlan,
    process_plan: ProcessTrackingPlan,
    clock: SessionClock,
    session_path: Option<PathBuf>,
    indexed_session_count: usize,
    last_persisted_session_path: Option<PathBuf>,
    last_session_summary: Option<SessionFileSummary>,
}

impl Recorder {
    pub fn new(config: AppConfig) -> Self {
        Self {
            session: SessionManifest::with_profile(config.profile.as_str()),
            config,
            state: RecorderState::Idle,
            capture_plan: CapturePlan::default(),
            process_plan: ProcessTrackingPlan::default(),
            clock: SessionClock::default(),
            session_path: None,
            indexed_session_count: 0,
            last_persisted_session_path: None,
            last_session_summary: None,
        }
    }

    pub fn status_snapshot(&self) -> RecorderStatus {
        let plan: RecordingPlan = self.config.recording_plan();
        let pending_issue = self.config.validate().err().map(render_validation_error);
        RecorderStatus {
            state: self.state,
            session_id: self.session.session_id.clone(),
            profile: self.config.profile,
            profile_summary: plan.profile_summary,
            screen_capture: plan.screen_capture_enabled,
            keyboard_events: plan.keyboard_events_enabled,
            mouse_events: plan.mouse_events_enabled,
            process_tracking: plan.process_tracking_enabled,
            encryption_enabled: plan.encryption_enabled,
            excluded_app_count: self.config.privacy_filters.excluded_apps.len(),
            allowed_app_count: self.config.privacy_filters.allowed_apps.len(),
            warnings: plan.warnings,
            pending_issue,
            session_event_count: self.session.events.len(),
            capture_backend: StubCaptureBackend.backend_name(),
            process_backend: StubProcessBackend.backend_name(),
            session_path: self.session_path.clone(),
            indexed_session_count: self.indexed_session_count,
            last_persisted_session_path: self.last_persisted_session_path.clone(),
            last_session_summary: self.last_session_summary.clone(),
        }
    }

    pub fn start(&mut self) -> Result<(), ConfigValidationError> {
        self.config.validate()?;

        if self.state == RecorderState::Recording {
            return Ok(());
        }

        self.clock.restart();
        self.session = SessionManifest::with_profile(self.config.profile.as_str());
        self.session_path = None;
        self.state = RecorderState::Recording;
        self.push_event(TimelineEventKind::RecordingStarted);

        if self.config.toggles.screen_capture {
            let capture = StubCaptureBackend;
            if let Some(frame) = capture.capture_frame_metadata(&self.capture_plan) {
                if frame.changed {
                    self.push_event(TimelineEventKind::FrameCaptured);
                }
            }
        }

        if self.config.toggles.process_tracking {
            let process = StubProcessBackend;
            if process.current_focus().is_some() && self.process_plan.track_focus_changes {
                self.push_event(TimelineEventKind::ProcessFocusChanged);
            }
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        if self.state == RecorderState::Idle {
            return;
        }

        self.state = RecorderState::Idle;
        self.push_event(TimelineEventKind::RecordingStopped);
    }

    pub fn persist_session(&mut self, root: &Path) -> io::Result<&Path> {
        let path = self.session.write_to_root(root)?;
        append_session_index(root, &self.session, &path)?;
        write_last_session_pointer(root, &path)?;
        self.session_path = Some(path);
        self.last_persisted_session_path = self.session_path.clone();
        self.indexed_session_count = read_session_index(root)?.len();
        self.last_session_summary = Some(SessionFileSummary {
            session_id: self.session.session_id.clone(),
            profile_name: self.session.profile_name.clone(),
            event_count: self.session.events.len(),
        });
        Ok(self
            .session_path
            .as_deref()
            .expect("session path should exist after persistence"))
    }

    pub fn has_session_data(&self) -> bool {
        !self.session.events.is_empty()
    }

    pub fn load_storage_state(&mut self, root: &Path) -> io::Result<()> {
        self.indexed_session_count = read_session_index(root)?.len();
        self.last_persisted_session_path = read_last_session_pointer(root)?;
        self.last_session_summary = match self.last_persisted_session_path.as_deref() {
            Some(path) => Some(read_session_file_summary(path)?),
            None => None,
        };
        Ok(())
    }

    pub fn last_indexed_session(&self, root: &Path) -> io::Result<Option<SessionIndexEntry>> {
        let mut entries = read_session_index(root)?;
        Ok(entries.pop())
    }

    fn push_event(&mut self, kind: TimelineEventKind) {
        let timestamp_ms = self.clock.now_ms();
        self.session.push_event(TimelineEvent::new_at(kind, timestamp_ms));
    }
}

fn render_validation_error(error: ConfigValidationError) -> &'static str {
    match error {
        ConfigValidationError::FullReplayRequiresAcknowledgement => {
            "full-replay profile requires explicit acknowledgement"
        }
        ConfigValidationError::KeyboardCaptureRequiresWarning => {
            "keyboard event capture requires explicit warning acknowledgement"
        }
        ConfigValidationError::FocusProfileRequiresExcludedOrAllowedApps => {
            "focus profile requires an allow-list or exclusion rule before start"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Recorder, RecorderState};
    use crate::config::{AppConfig, RecordingProfile, SourceToggles};
    use std::path::Path;

    #[test]
    fn recorder_starts_idle() {
        let recorder = Recorder::new(AppConfig::default());
        let status = recorder.status_snapshot();
        assert_eq!(status.state, RecorderState::Idle);
    }

    #[test]
    fn privacy_profile_can_start_recording() {
        let mut recorder = Recorder::new(AppConfig::default());
        recorder.start().expect("privacy profile should start");
        let status = recorder.status_snapshot();
        assert_eq!(status.state, RecorderState::Recording);
        assert_eq!(status.session_event_count, 2);
    }

    #[test]
    fn full_replay_is_blocked_without_acknowledgement() {
        let config = AppConfig {
            profile: RecordingProfile::FullReplay,
            toggles: SourceToggles::for_profile(RecordingProfile::FullReplay),
            ..AppConfig::default()
        };
        let mut recorder = Recorder::new(config);

        assert!(recorder.start().is_err());
        assert_eq!(recorder.status_snapshot().state, RecorderState::Idle);
    }

    #[test]
    fn recorder_persists_session_path() {
        let mut recorder = Recorder::new(AppConfig::default());
        let root = Path::new("data/test-sessions");
        let path = recorder
            .persist_session(root)
            .expect("session persistence should succeed");
        assert!(path.to_string_lossy().contains("privacy-session-"));
        assert!(path.to_string_lossy().ends_with(".log"));
    }

    #[test]
    fn recorder_loads_persisted_storage_state() {
        let mut recorder = Recorder::new(AppConfig::default());
        let root = Path::new("data/test-storage-state");
        recorder.start().expect("start should succeed");
        recorder
            .persist_session(root)
            .expect("session persistence should succeed");

        let mut fresh = Recorder::new(AppConfig::default());
        fresh
            .load_storage_state(root)
            .expect("storage state load should succeed");
        let status = fresh.status_snapshot();

        assert_eq!(status.indexed_session_count, 1);
        assert!(status
            .last_persisted_session_path
            .expect("last session path should exist")
            .to_string_lossy()
            .contains("privacy-session-"));
        assert_eq!(
            status
                .last_session_summary
                .expect("last session summary should exist")
                .profile_name,
            "privacy"
        );
    }
}
