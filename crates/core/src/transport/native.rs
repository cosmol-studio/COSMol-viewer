use super::Result;
use crate::{Feedback, Request};
use eframe::egui::Context;
use iceoryx2::{
    node::{Node, NodeBuilder},
    port::{notifier::Notifier, publisher::Publisher, subscriber::Subscriber},
    service::{ipc, port_factory::publish_subscribe::PortFactory},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

type Sender = Publisher<ipc::Service, [u8], ()>;
type Receiver = Subscriber<ipc::Service, [u8], ()>;
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn services(
    name: &str,
) -> (
    Node<ipc::Service>,
    PortFactory<ipc::Service, [u8], ()>,
    PortFactory<ipc::Service, [u8], ()>,
) {
    iceoryx2::prelude::set_log_level(iceoryx2::prelude::LogLevel::Error);
    let node = NodeBuilder::new().create::<ipc::Service>().unwrap();
    let scenes = node
        .service_builder(&format!("{name}/scene-v2").as_str().try_into().unwrap())
        .publish_subscribe::<[u8]>()
        .subscriber_max_buffer_size(4)
        .subscriber_max_borrowed_samples(2)
        .enable_safe_overflow(false)
        .open_or_create()
        .unwrap();
    let events = node
        .service_builder(&format!("{name}/feedback-v2").as_str().try_into().unwrap())
        .publish_subscribe::<[u8]>()
        .subscriber_max_buffer_size(16)
        .enable_safe_overflow(false)
        .open_or_create()
        .unwrap();
    (node, scenes, events)
}

fn send(tx: &Sender, message: impl Serialize) -> Result<()> {
    let bytes = postcard::to_allocvec(&(VERSION, message)).map_err(|e| e.to_string())?;
    let mut sample = tx.loan_slice(bytes.len()).map_err(|e| e.to_string())?;
    sample.payload_mut().copy_from_slice(&bytes);
    if sample.send().map_err(|e| e.to_string())? == 0 {
        return Err("Transport has no receiver".into());
    }
    Ok(())
}

fn receive<T: DeserializeOwned>(rx: &Receiver) -> Result<Option<T>> {
    let Some(sample) = rx.receive().map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let (version, message): (&str, T) =
        postcard::from_bytes(sample.payload()).map_err(|e| e.to_string())?;
    if version != VERSION {
        return Err(format!(
            "transport protocol mismatch: {version} != {VERSION}"
        ));
    }
    Ok(Some(message))
}

/// Producer endpoint. Only this backend knows about iceoryx2 ports and samples.
pub struct ViewerTransport {
    tx: Sender,
    rx: Receiver,
    repaint: Notifier<ipc::Service>,
}

impl ViewerTransport {
    pub fn connect(name: &str) -> Self {
        let (_node, scenes, events) = services(name);
        let repaint = _node
            .service_builder(&format!("{name}/repaint-v1").as_str().try_into().unwrap())
            .event()
            .open_or_create()
            .unwrap();
        Self {
            tx: scenes
                .publisher_builder()
                .initial_max_slice_len(4096)
                .allocation_strategy(iceoryx2::prelude::AllocationStrategy::PowerOfTwo)
                .create()
                .unwrap(),
            rx: events.subscriber_builder().create().unwrap(),
            repaint: repaint.notifier_builder().create().unwrap(),
        }
    }

    pub fn send_request(&self, request: Request<impl Serialize>) -> Result<()> {
        // Publish first so a woken renderer always sees the queued request.
        send(&self.tx, request)?;
        self.repaint
            .notify()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn try_feedback(&self) -> Result<Option<Feedback>> {
        receive(&self.rx)
    }
}

/// Rendering endpoint, owned exclusively by the App after initialization.
pub struct RendererTransport {
    rx: Receiver,
    tx: Sender,
    name: String,
    repaint_worker: RefCell<Option<RepaintWorker>>,
}

struct RepaintWorker {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for RepaintWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl RendererTransport {
    pub fn connect(name: &str) -> Self {
        let (_node, scenes, events) = services(name);
        Self {
            name: name.into(),
            repaint_worker: RefCell::new(None),
            rx: scenes.subscriber_builder().create().unwrap(),
            tx: events
                .publisher_builder()
                .initial_max_slice_len(64)
                .allocation_strategy(iceoryx2::prelude::AllocationStrategy::PowerOfTwo)
                .create()
                .unwrap(),
        }
    }

    pub fn try_request(&self) -> Result<Option<Request>> {
        receive(&self.rx)
    }

    pub fn send_feedback(&self, feedback: Feedback) -> Result<()> {
        send(&self.tx, feedback)
    }

    pub fn attach_repaint(&self, ctx: Context) {
        let name = self.name.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker_ctx = ctx.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(0);
        let worker = thread::Builder::new()
            .name("cosmol-viewer-repaint".into())
            .spawn(move || {
                // Non-Send iceoryx handles are created and kept on this thread.
                let node = NodeBuilder::new().create::<ipc::Service>().unwrap();
                let event = node
                    .service_builder(&format!("{name}/repaint-v1").as_str().try_into().unwrap())
                    .event()
                    .open_or_create()
                    .unwrap();
                let listener = event.listener_builder().create().unwrap();
                if ready_tx.send(()).is_err() {
                    return;
                }
                while !worker_stop.load(Ordering::Acquire) {
                    // Timeout only checks shutdown; it never requests a repaint.
                    let notifications = listener
                        .timed_wait(|_| {}, Duration::from_millis(100))
                        .unwrap();
                    if notifications > 0 && !worker_stop.load(Ordering::Acquire) {
                        worker_ctx.request_repaint();
                    }
                }
            })
            .expect("Failed to start renderer repaint listener");
        let worker = RepaintWorker {
            stop,
            thread: Some(worker),
        };
        ready_rx
            .recv()
            .expect("Renderer repaint listener failed to initialize");
        *self.repaint_worker.borrow_mut() = Some(worker);
        // Commands sent before listener attachment may have no notification.
        // This one initial wake ensures they are processed without polling.
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_scene_requests_keep_owned_wire_format() {
        let scene = crate::scene::Scene {
            scale: 2.5,
            viewport: Some([800, 500]),
            scene_center: [1.0, 2.0, 3.0],
            ..Default::default()
        };
        let requests = [
            (
                Request::UpdateScene {
                    request_id: 7,
                    scene: &scene,
                },
                Request::UpdateScene {
                    request_id: 7,
                    scene: scene.clone(),
                },
            ),
            (
                Request::InitializeScene {
                    request_id: 0,
                    scene: &scene,
                    width: 800.0,
                    height: 500.0,
                },
                Request::InitializeScene {
                    request_id: 0,
                    scene: scene.clone(),
                    width: 800.0,
                    height: 500.0,
                },
            ),
        ];
        for (borrowed, owned) in requests {
            let bytes = postcard::to_allocvec(&(VERSION, borrowed)).unwrap();
            assert_eq!(bytes, postcard::to_allocvec(&(VERSION, &owned)).unwrap());
            let (version, decoded): (&str, Request) = postcard::from_bytes(&bytes).unwrap();
            assert_eq!(version, VERSION);
            assert_eq!(
                serde_json::to_value(decoded).unwrap(),
                serde_json::to_value(owned).unwrap()
            );
        }
    }

    #[test]
    fn native_repaint_is_event_driven_and_handles_pre_attachment_requests() {
        let name = format!(
            "cosmol-repaint-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let viewer = ViewerTransport::connect(&name);
        let renderer = RendererTransport::connect(&name);
        let ctx = Context::default();
        for _ in 0..3 {
            let _ = ctx.run_ui(Default::default(), |_| {});
        }
        let (wake_tx, wake_rx) = mpsc::channel();
        ctx.set_request_repaint_callback(move |_| {
            let _ = wake_tx.send(());
        });
        viewer
            .send_request(Request::<crate::scene::Scene>::ShowFps {
                request_id: 1,
                enabled: true,
            })
            .unwrap();
        assert!(wake_rx.try_recv().is_err());
        renderer.attach_repaint(ctx.clone());
        wake_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(
            renderer.try_request().unwrap(),
            Some(Request::ShowFps { request_id: 1, .. })
        ));
        for _ in 0..3 {
            let _ = ctx.run_ui(Default::default(), |_| {});
        }
        while wake_rx.try_recv().is_ok() {}
        assert!(
            matches!(
                wake_rx.recv_timeout(Duration::from_millis(250)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ),
            "An idle IPC listener must not request repainting"
        );
        viewer
            .send_request(Request::<crate::scene::Scene>::ShowCameraParameters {
                request_id: 2,
                enabled: true,
            })
            .unwrap();
        wake_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(
            renderer.try_request().unwrap(),
            Some(Request::ShowCameraParameters { request_id: 2, .. })
        ));
        let dropped_at = std::time::Instant::now();
        drop(renderer);
        assert!(dropped_at.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn native_endpoints_round_trip_typed_messages() {
        let name = format!(
            "cosmol-transport-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let viewer = ViewerTransport::connect(&name);
        let renderer = RendererTransport::connect(&name);
        assert!(renderer.try_request().unwrap().is_none());
        assert!(viewer.try_feedback().unwrap().is_none());
        let mut scene = crate::scene::Scene {
            scale: 2.5,
            ..Default::default()
        };
        viewer
            .send_request(Request::UpdateScene {
                request_id: 7,
                scene: &scene,
            })
            .unwrap();
        scene.scale = 9.0;
        assert!(matches!(
            renderer.try_request().unwrap(),
            Some(Request::UpdateScene { request_id: 7, scene }) if scene.scale == 2.5
        ));
        renderer
            .send_feedback(Feedback::Applied { request_id: 7 })
            .unwrap();
        assert_eq!(
            viewer.try_feedback().unwrap(),
            Some(Feedback::Applied { request_id: 7 })
        );
        let screenshot = Feedback::ScreenshotTaken {
            request_id: 8,
            width: 256,
            height: 256,
            rgba: vec![127; 256 * 256 * 4],
        };
        renderer.send_feedback(screenshot).unwrap();
        assert!(matches!(viewer.try_feedback().unwrap(),
            Some(Feedback::ScreenshotTaken { request_id: 8, rgba, .. }) if rgba.len() == 256 * 256 * 4));
        renderer.send_feedback(Feedback::Closed).unwrap();
        assert_eq!(viewer.try_feedback().unwrap(), Some(Feedback::Closed));
    }
}
