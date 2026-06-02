use std::collections::HashMap;

/// Command/cap advertisement aligned with macOS MacNodeModeCoordinator defaults.
pub fn linux_node_advertisement(
    camera_enabled: bool,
    location_enabled: bool,
    screen_enabled: bool,
    talk_enabled: bool,
) -> (Vec<String>, Vec<String>, HashMap<String, bool>) {
    let mut caps = vec![
        "canvas".into(),
        "screen".into(),
        "system".into(),
    ];
    let mut commands = vec![
        "canvas.present".into(),
        "canvas.hide".into(),
        "canvas.navigate".into(),
        "canvas.eval".into(),
        "canvas.snapshot".into(),
        "canvas.a2ui.push".into(),
        "canvas.a2ui.pushJSONL".into(),
        "canvas.a2ui.reset".into(),
        "screen.snapshot".into(),
        "system.notify".into(),
        "system.which".into(),
        "system.run".into(),
        "system.run.prepare".into(),
        "system.execApprovals.get".into(),
        "system.execApprovals.set".into(),
    ];
    let mut permissions = HashMap::from([
        ("screen.record".into(), false),
        ("camera.capture".into(), false),
    ]);

    if screen_enabled {
        commands.push("screen.record".into());
        permissions.insert("screen.record".into(), true);
    }
    if camera_enabled {
        caps.push("camera".into());
        commands.push("camera.list".into());
        commands.push("camera.snap".into());
        commands.push("camera.clip".into());
        permissions.insert("camera.capture".into(), true);
    }
    if location_enabled {
        caps.push("location".into());
        commands.push("location.get".into());
    }
    if talk_enabled {
        caps.push("talk".into());
        commands.push("talk.ptt.start".into());
        commands.push("talk.ptt.stop".into());
        commands.push("talk.ptt.cancel".into());
        commands.push("talk.ptt.once".into());
    }

    (caps, commands, permissions)
}

#[cfg(test)]
mod tests {
    use super::linux_node_advertisement;

    #[test]
    fn advertisement_includes_camera_when_enabled() {
        let (caps, commands, permissions) = linux_node_advertisement(true, false, false, false);
        assert!(caps.contains(&"camera".to_string()));
        assert!(commands.contains(&"camera.clip".to_string()));
        assert_eq!(permissions.get("camera.capture"), Some(&true));
    }

    #[test]
    fn advertisement_omits_camera_when_disabled() {
        let (caps, commands, permissions) = linux_node_advertisement(false, false, true, false);
        assert!(!caps.contains(&"camera".to_string()));
        assert!(!commands.iter().any(|c| c.starts_with("camera.")));
        assert_eq!(permissions.get("camera.capture"), Some(&false));
        assert!(commands.contains(&"screen.record".to_string()));
    }

    #[test]
    fn advertisement_includes_talk_when_enabled() {
        let (_, commands, _) = linux_node_advertisement(false, false, false, true);
        assert!(commands.contains(&"talk.ptt.start".to_string()));
    }
}
