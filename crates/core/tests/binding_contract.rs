use cosmol_viewer_core::binding_contract::{
    BINDING_CONTRACT, BindingReceiver, RuntimePlatform, to_json,
};

#[test]
fn native_lifecycle_is_registered_but_not_exported_to_browsers() {
    let wait = BINDING_CONTRACT
        .iter()
        .find(|row| row.semantic_id == "Viewer.keep_alive")
        .unwrap();
    assert_eq!(wait.receiver, BindingReceiver::Owned);
    assert_eq!(wait.rust_platforms, &[RuntimePlatform::Native]);
    assert_eq!(wait.python.name, Some("Viewer.keep_alive"));
    assert_eq!(wait.python.platforms, &[RuntimePlatform::Native]);
    assert_eq!(wait.javascript.name, None);
    assert!(wait.javascript.platforms.is_empty());
    assert!(!wait.javascript.unsupported_reason.unwrap().is_empty());

    let status = BINDING_CONTRACT
        .iter()
        .find(|row| row.semantic_id == "Viewer.is_open")
        .unwrap();
    assert_eq!(status.receiver, BindingReceiver::Shared);
    assert_eq!(status.python.name, Some("Viewer.is_open"));
    assert_eq!(status.python.platforms, &[RuntimePlatform::Native]);
    assert_eq!(status.javascript.name, None);
}

#[test]
fn registry_has_unique_ids_and_explicit_projection_availability() {
    let mut ids = std::collections::HashSet::new();
    for row in BINDING_CONTRACT {
        assert!(ids.insert(row.semantic_id));
        assert!(!row.rust_platforms.is_empty());
        let mut defaults = std::collections::HashSet::new();
        for default in row.defaults {
            assert!(defaults.insert(default.parameter));
            assert!(
                row.parameters
                    .iter()
                    .any(|parameter| parameter.name == default.parameter)
            );
        }
        for projection in [row.python, row.javascript] {
            assert_eq!(projection.name.is_some(), !projection.platforms.is_empty());
            assert_eq!(
                projection.name.is_none(),
                projection.unsupported_reason.is_some()
            );
        }
    }
    let decoded: serde_json::Value = serde_json::from_str(&to_json()).unwrap();
    assert_eq!(decoded.as_array().unwrap().len(), BINDING_CONTRACT.len());
}

#[test]
fn notebook_commands_use_the_registered_shared_dispatch_endpoint() {
    for (id, command) in [
        ("Viewer.update", "UpdateScene"),
        ("Viewer.camera_parameter_logging", "CameraParameterLogging"),
        ("Viewer.show_fps", "ShowFps"),
        ("Viewer.show_camera_parameters", "ShowCameraParameters"),
    ] {
        let row = BINDING_CONTRACT
            .iter()
            .find(|row| row.semantic_id == id)
            .unwrap();
        assert_ne!(row.javascript.name, Some("Viewer.dispatch"));
        assert_eq!(row.javascript.notebook_endpoint, Some("Viewer.dispatch"));
        assert_eq!(row.javascript.command, Some(command));
    }
}

#[test]
fn browser_screenshot_has_no_notebook_return_endpoint() {
    let screenshot = BINDING_CONTRACT
        .iter()
        .find(|row| row.semantic_id == "Viewer.take_screenshot")
        .unwrap();
    assert_eq!(screenshot.javascript.name, Some("Viewer.takeScreenshot"));
    assert_eq!(screenshot.javascript.platforms, &[RuntimePlatform::Browser]);
    assert_eq!(screenshot.javascript.notebook_endpoint, None);
    assert_eq!(screenshot.javascript.command, None);
}
