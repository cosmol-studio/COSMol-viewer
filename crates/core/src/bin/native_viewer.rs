#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let name = match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--child"), Some(name), None) => name,
        _ => {
            eprintln!("Usage: cosmol-viewer-native --child <IPC service name>");
            return std::process::ExitCode::FAILURE;
        }
    };
    match cosmol_viewer_core::run_render_child(&name) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Native viewer failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}
