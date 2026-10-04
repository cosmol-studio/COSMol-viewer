use cosmol_viewer_core::{binding_contract::BINDING_CONTRACT, scene::Scene};
use cosmol_viewer_wasm::protocol::ViewerCommand;

#[test]
fn registered_dispatch_variants_match_the_actual_wire_commands() {
    for (id, command) in [
        (
            "Viewer.update",
            ViewerCommand::UpdateScene {
                scene: Scene::new(),
            },
        ),
        (
            "Viewer.set_camera_parameter_logging",
            ViewerCommand::SetCameraParameterLogging { enabled: true },
        ),
    ] {
        let row = BINDING_CONTRACT
            .iter()
            .find(|row| row.semantic_id == id)
            .unwrap();
        let serialized = serde_json::to_value(command).unwrap();
        let variant = serialized.as_object().unwrap().keys().next().unwrap();
        assert_eq!(row.javascript.command, Some(variant.as_str()));
    }
}
