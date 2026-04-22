#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingProfile {
    Minimal,
    Focus,
    Privacy,
    FullReplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValidationError {
    FullReplayRequiresAcknowledgement,
    KeyboardCaptureRequiresWarning,
    FocusProfileRequiresExcludedOrAllowedApps,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigParseError {
    MissingValue(&'static str),
    UnknownFlag(String),
    UnknownProfile(String),
    UnknownCommand(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Start,
    Status,
    Stop,
}

impl RecordingProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordingProfile::Minimal => "minimal",
            RecordingProfile::Focus => "focus",
            RecordingProfile::Privacy => "privacy",
            RecordingProfile::FullReplay => "full-replay",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            RecordingProfile::Minimal => "Screen-only capture with the smallest data surface.",
            RecordingProfile::Focus => "Capture work on selected apps with safe event context.",
            RecordingProfile::Privacy => "Default mode with aggressive privacy boundaries.",
            RecordingProfile::FullReplay => {
                "Advanced mode with all sources enabled and explicit user warnings."
            }
        }
    }

    pub fn warnings(self) -> &'static [&'static str] {
        match self {
            RecordingProfile::Minimal => &[],
            RecordingProfile::Focus => &["Requires explicit allow-list configuration for target apps."],
            RecordingProfile::Privacy => &[
                "Private browsing and password-entry contexts must remain excluded where detectable.",
            ],
            RecordingProfile::FullReplay => &[
                "Keyboard collection must remain opt-in and clearly disclosed.",
                "This mode should never be the default profile.",
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceToggles {
    pub screen_capture: bool,
    pub keyboard_events: bool,
    pub mouse_events: bool,
    pub process_tracking: bool,
}

impl SourceToggles {
    pub fn for_profile(profile: RecordingProfile) -> Self {
        match profile {
            RecordingProfile::Minimal => Self {
                screen_capture: true,
                keyboard_events: false,
                mouse_events: false,
                process_tracking: false,
            },
            RecordingProfile::Focus => Self {
                screen_capture: true,
                keyboard_events: false,
                mouse_events: true,
                process_tracking: true,
            },
            RecordingProfile::Privacy => Self {
                screen_capture: true,
                keyboard_events: false,
                mouse_events: true,
                process_tracking: true,
            },
            RecordingProfile::FullReplay => Self {
                screen_capture: true,
                keyboard_events: true,
                mouse_events: true,
                process_tracking: true,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyFilters {
    pub excluded_apps: Vec<String>,
    pub allowed_apps: Vec<String>,
    pub exclude_incognito_windows: bool,
    pub exclude_password_contexts: bool,
}

impl Default for PrivacyFilters {
    fn default() -> Self {
        Self {
            excluded_apps: Vec::new(),
            allowed_apps: Vec::new(),
            exclude_incognito_windows: true,
            exclude_password_contexts: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub profile: RecordingProfile,
    pub toggles: SourceToggles,
    pub privacy_filters: PrivacyFilters,
    pub encryption_enabled: bool,
    pub full_replay_acknowledged: bool,
    pub keyboard_warning_acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingPlan {
    pub profile_name: &'static str,
    pub profile_summary: &'static str,
    pub warnings: Vec<&'static str>,
    pub screen_capture_enabled: bool,
    pub keyboard_events_enabled: bool,
    pub mouse_events_enabled: bool,
    pub process_tracking_enabled: bool,
    pub encryption_enabled: bool,
    pub privacy_filter_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let profile = RecordingProfile::Privacy;
        Self {
            profile,
            toggles: SourceToggles::for_profile(profile),
            privacy_filters: PrivacyFilters::default(),
            encryption_enabled: false,
            full_replay_acknowledged: false,
            keyboard_warning_acknowledged: false,
        }
    }
}

impl AppConfig {
    pub fn from_args(args: &[String]) -> Result<(Command, Self), ConfigParseError> {
        let mut config = Self::default();
        let mut index = 0;
        let mut command = Command::Status;

        if let Some(first) = args.first() {
            if !first.starts_with("--") {
                command = parse_command(first)?;
                index = 1;
            }
        }

        while index < args.len() {
            match args[index].as_str() {
                "--profile" => {
                    let value = args
                        .get(index + 1)
                        .ok_or(ConfigParseError::MissingValue("--profile"))?;
                    config.profile = parse_profile(value)?;
                    config.toggles = SourceToggles::for_profile(config.profile);
                    index += 2;
                }
                "--exclude-app" => {
                    let value = args
                        .get(index + 1)
                        .ok_or(ConfigParseError::MissingValue("--exclude-app"))?;
                    config.privacy_filters.excluded_apps.push(value.clone());
                    index += 2;
                }
                "--allow-app" => {
                    let value = args
                        .get(index + 1)
                        .ok_or(ConfigParseError::MissingValue("--allow-app"))?;
                    config.privacy_filters.allowed_apps.push(value.clone());
                    index += 2;
                }
                "--enable-encryption" => {
                    config.encryption_enabled = true;
                    index += 1;
                }
                "--ack-full-replay" => {
                    config.full_replay_acknowledged = true;
                    index += 1;
                }
                "--ack-keyboard-warning" => {
                    config.keyboard_warning_acknowledged = true;
                    index += 1;
                }
                flag => {
                    return Err(ConfigParseError::UnknownFlag(flag.to_string()));
                }
            }
        }

        Ok((command, config))
    }

    pub fn recording_plan(&self) -> RecordingPlan {
        RecordingPlan {
            profile_name: self.profile.as_str(),
            profile_summary: self.profile.summary(),
            warnings: self.profile.warnings().to_vec(),
            screen_capture_enabled: self.toggles.screen_capture,
            keyboard_events_enabled: self.toggles.keyboard_events,
            mouse_events_enabled: self.toggles.mouse_events,
            process_tracking_enabled: self.toggles.process_tracking,
            encryption_enabled: self.encryption_enabled,
            privacy_filter_enabled: self.privacy_filters.exclude_incognito_windows
                || self.privacy_filters.exclude_password_contexts
                || !self.privacy_filters.excluded_apps.is_empty()
                || !self.privacy_filters.allowed_apps.is_empty(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.profile == RecordingProfile::FullReplay && !self.full_replay_acknowledged {
            return Err(ConfigValidationError::FullReplayRequiresAcknowledgement);
        }

        if self.toggles.keyboard_events && !self.keyboard_warning_acknowledged {
            return Err(ConfigValidationError::KeyboardCaptureRequiresWarning);
        }

        if self.profile == RecordingProfile::Focus
            && self.privacy_filters.allowed_apps.is_empty()
            && self.privacy_filters.excluded_apps.is_empty()
        {
            return Err(ConfigValidationError::FocusProfileRequiresExcludedOrAllowedApps);
        }

        Ok(())
    }
}

fn parse_profile(value: &str) -> Result<RecordingProfile, ConfigParseError> {
    match value {
        "minimal" => Ok(RecordingProfile::Minimal),
        "focus" => Ok(RecordingProfile::Focus),
        "privacy" => Ok(RecordingProfile::Privacy),
        "full-replay" => Ok(RecordingProfile::FullReplay),
        other => Err(ConfigParseError::UnknownProfile(other.to_string())),
    }
}

fn parse_command(value: &str) -> Result<Command, ConfigParseError> {
    match value {
        "start" => Ok(Command::Start),
        "status" => Ok(Command::Status),
        "stop" => Ok(Command::Stop),
        other => Err(ConfigParseError::UnknownCommand(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, Command, ConfigParseError, ConfigValidationError, RecordingProfile,
        SourceToggles,
    };

    #[test]
    fn default_config_uses_privacy_profile() {
        let config = AppConfig::default();
        assert_eq!(config.profile, RecordingProfile::Privacy);
        assert!(!config.toggles.keyboard_events);
    }

    #[test]
    fn minimal_profile_is_screen_only() {
        let toggles = SourceToggles::for_profile(RecordingProfile::Minimal);
        assert!(toggles.screen_capture);
        assert!(!toggles.keyboard_events);
        assert!(!toggles.mouse_events);
        assert!(!toggles.process_tracking);
    }

    #[test]
    fn full_replay_requires_explicit_keyboard_enablement() {
        let toggles = SourceToggles::for_profile(RecordingProfile::FullReplay);
        assert!(toggles.keyboard_events);
    }

    #[test]
    fn privacy_profile_carries_safety_warning() {
        let config = AppConfig::default();
        let plan = config.recording_plan();
        assert_eq!(plan.profile_name, "privacy");
        assert_eq!(plan.warnings.len(), 1);
    }

    #[test]
    fn full_replay_requires_explicit_acknowledgement() {
        let config = AppConfig {
            profile: RecordingProfile::FullReplay,
            toggles: SourceToggles::for_profile(RecordingProfile::FullReplay),
            ..AppConfig::default()
        };

        assert_eq!(
            config.validate(),
            Err(ConfigValidationError::FullReplayRequiresAcknowledgement)
        );
    }

    #[test]
    fn focus_profile_requires_scope_rules() {
        let config = AppConfig {
            profile: RecordingProfile::Focus,
            toggles: SourceToggles::for_profile(RecordingProfile::Focus),
            ..AppConfig::default()
        };

        assert_eq!(
            config.validate(),
            Err(ConfigValidationError::FocusProfileRequiresExcludedOrAllowedApps)
        );
    }

    #[test]
    fn args_parser_accepts_focus_profile_and_allow_list() {
        let args = vec![
            "start".to_string(),
            "--profile".to_string(),
            "focus".to_string(),
            "--allow-app".to_string(),
            "code.exe".to_string(),
        ];
        let (command, config) = AppConfig::from_args(&args).expect("args should parse");

        assert_eq!(command, Command::Start);
        assert_eq!(config.profile, RecordingProfile::Focus);
        assert_eq!(config.privacy_filters.allowed_apps, vec!["code.exe"]);
    }

    #[test]
    fn args_parser_rejects_unknown_profile() {
        let args = vec!["--profile".to_string(), "invalid".to_string()];
        assert_eq!(
            AppConfig::from_args(&args),
            Err(ConfigParseError::UnknownProfile("invalid".to_string()))
        );
    }

    #[test]
    fn args_parser_defaults_to_status_command() {
        let args = vec!["--enable-encryption".to_string()];
        let (command, config) = AppConfig::from_args(&args).expect("args should parse");

        assert_eq!(command, Command::Status);
        assert!(config.encryption_enabled);
    }
}
