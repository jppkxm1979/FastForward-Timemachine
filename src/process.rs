#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessTrackingPlan {
    pub track_focus_changes: bool,
    pub track_start_stop_events: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSnapshot {
    pub process_name: String,
    pub is_focused: bool,
}

pub trait ProcessBackend {
    fn backend_name(&self) -> &'static str;
    fn current_focus(&self) -> Option<ProcessSnapshot>;
}

#[derive(Debug, Default)]
pub struct StubProcessBackend;

impl ProcessBackend for StubProcessBackend {
    fn backend_name(&self) -> &'static str {
        "stub-process"
    }

    fn current_focus(&self) -> Option<ProcessSnapshot> {
        Some(ProcessSnapshot {
            process_name: "explorer.exe".to_string(),
            is_focused: true,
        })
    }
}

impl Default for ProcessTrackingPlan {
    fn default() -> Self {
        Self {
            track_focus_changes: true,
            track_start_stop_events: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ProcessBackend, StubProcessBackend};

    #[test]
    fn stub_process_backend_reports_focus() {
        let backend = StubProcessBackend;
        let focus = backend.current_focus().expect("focus should exist");
        assert_eq!(focus.process_name, "explorer.exe");
        assert!(focus.is_focused);
    }
}
