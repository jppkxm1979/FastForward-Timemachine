#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSafetyPolicy {
    pub store_raw_keyboard_text: bool,
    pub store_keyboard_timing_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSnapshot {
    pub keyboard_metadata_enabled: bool,
    pub mouse_metadata_enabled: bool,
}

impl Default for InputSafetyPolicy {
    fn default() -> Self {
        Self {
            store_raw_keyboard_text: false,
            store_keyboard_timing_only: true,
        }
    }
}

impl InputSafetyPolicy {
    pub fn snapshot(&self, mouse_enabled: bool) -> InputSnapshot {
        InputSnapshot {
            keyboard_metadata_enabled: self.store_keyboard_timing_only,
            mouse_metadata_enabled: mouse_enabled,
        }
    }
}
