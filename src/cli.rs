use crate::recorder::{RecorderState, RecorderStatus};

pub fn render_status(status: &RecorderStatus) -> String {
    let mut output = format!(
        concat!(
            "FastForward-Timemachine\n",
            "state: {}\n",
            "session_id: {}\n",
            "profile: {}\n",
            "summary: {}\n",
            "screen_capture: {}\n",
            "keyboard_events: {}\n",
            "mouse_events: {}\n",
            "process_tracking: {}\n",
            "encryption: {}\n",
            "capture_backend: {}\n",
            "process_backend: {}\n",
            "privacy_filters: {}\n",
            "excluded_apps: {}\n",
            "allowed_apps: {}\n",
            "session_events: {}\n",
            "session_path: {}\n",
        ),
        render_state(status.state),
        status.session_id,
        status.profile.as_str(),
        status.profile_summary,
        on_off(status.screen_capture),
        on_off(status.keyboard_events),
        on_off(status.mouse_events),
        on_off(status.process_tracking),
        on_off(status.encryption_enabled),
        status.capture_backend,
        status.process_backend,
        if status.excluded_app_count > 0 || status.allowed_app_count > 0 {
            "ON"
        } else {
            "BASELINE"
        },
        status.excluded_app_count,
        status.allowed_app_count,
        status.session_event_count,
        render_session_path(status.session_path.as_deref()),
    );

    if let Some(issue) = status.pending_issue {
        output.push_str("pending_issue: ");
        output.push_str(issue);
        output.push('\n');
    }

    for warning in &status.warnings {
        output.push_str("warning: ");
        output.push_str(warning);
        output.push('\n');
    }

    output
}

fn render_session_path(path: Option<&std::path::Path>) -> String {
    match path {
        Some(path) => path.display().to_string(),
        None => "unwritten".to_string(),
    }
}

fn render_state(state: RecorderState) -> &'static str {
    match state {
        RecorderState::Idle => "OFF",
        RecorderState::Recording => "ON",
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

#[cfg(test)]
mod tests {
    use super::render_status;
    use crate::config::AppConfig;
    use crate::recorder::Recorder;

    #[test]
    fn status_output_shows_privacy_defaults() {
        let recorder = Recorder::new(AppConfig::default());
        let output = render_status(&recorder.status_snapshot());

        assert!(output.contains("profile: privacy"));
        assert!(output.contains("session_id: session-"));
        assert!(output.contains("keyboard_events: OFF"));
        assert!(output.contains("session_events: 0"));
        assert!(output.contains("capture_backend: stub-capture"));
        assert!(output.contains("session_path: unwritten"));
        assert!(output.contains("warning:"));
    }
}
