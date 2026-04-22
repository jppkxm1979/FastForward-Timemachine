#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePlan {
    pub interval_ms: u64,
    pub delta_detection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureTarget {
    pub display_name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMetadata {
    pub target_display_name: String,
    pub changed: bool,
    pub timestamp_ms: u64,
}

pub trait CaptureBackend {
    fn backend_name(&self) -> &'static str;
    fn enumerate_targets(&self) -> Vec<CaptureTarget>;
    fn capture_frame_metadata(&self, plan: &CapturePlan) -> Option<FrameMetadata>;
}

#[derive(Debug, Default)]
pub struct StubCaptureBackend;

impl CaptureBackend for StubCaptureBackend {
    fn backend_name(&self) -> &'static str {
        "stub-capture"
    }

    fn enumerate_targets(&self) -> Vec<CaptureTarget> {
        vec![CaptureTarget {
            display_name: "primary-display".to_string(),
            width: 1920,
            height: 1080,
        }]
    }

    fn capture_frame_metadata(&self, _plan: &CapturePlan) -> Option<FrameMetadata> {
        Some(FrameMetadata {
            target_display_name: "primary-display".to_string(),
            changed: false,
            timestamp_ms: 0,
        })
    }
}

impl Default for CapturePlan {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            delta_detection: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureBackend, CapturePlan, StubCaptureBackend};

    #[test]
    fn stub_backend_exposes_primary_display() {
        let backend = StubCaptureBackend;
        let targets = backend.enumerate_targets();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].display_name, "primary-display");
        assert!(backend.capture_frame_metadata(&CapturePlan::default()).is_some());
    }
}
