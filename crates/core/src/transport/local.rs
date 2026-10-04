use super::Result;
use crate::{Feedback, Request};
use eframe::egui::Context;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context as TaskContext, Poll, Waker},
};

#[derive(Default)]
struct State {
    requests: VecDeque<Request>,
    feedback: VecDeque<Feedback>,
    closed: bool,
    repaint: Option<Context>,
    waiters: HashMap<u64, Waker>,
}

/// Browser transport carries typed messages, not emulated IPC byte samples.
#[derive(Clone)]
pub struct ViewerTransport(Rc<RefCell<State>>);

pub struct RendererTransport(Rc<RefCell<State>>);

pub fn pair() -> (ViewerTransport, RendererTransport) {
    let state = Rc::new(RefCell::new(State::default()));
    (ViewerTransport(state.clone()), RendererTransport(state))
}

fn close(state: &RefCell<State>) {
    let waiters = {
        let mut state = state.borrow_mut();
        state.closed = true;
        state.requests.clear();
        state.feedback.clear();
        state.repaint = None;
        std::mem::take(&mut state.waiters)
    };
    for waker in waiters.into_values() {
        waker.wake();
    }
}

impl ViewerTransport {
    pub fn send_request(&self, request: Request) -> Result<()> {
        let repaint = {
            let mut state = self.0.borrow_mut();
            if state.closed {
                return Err("Viewer is closed".into());
            }
            // Sync updates enqueue commands; acknowledgments are housekeeping.
            // Preserve screenshot replies for their independent async waiters.
            state.feedback.retain(|event| {
                !matches!(
                    event,
                    Feedback::Applied { .. } | Feedback::Initialized { .. }
                )
            });
            state.requests.push_back(request);
            state.repaint.clone()
        };
        if let Some(ctx) = repaint {
            ctx.request_repaint();
        }
        Ok(())
    }

    pub fn try_feedback(&self) -> Result<Option<Feedback>> {
        let mut state = self.0.borrow_mut();
        if state.closed {
            return Err("Viewer is closed".into());
        }
        Ok(state.feedback.pop_front())
    }

    pub async fn wait_screenshot(&self, request_id: u64) -> Result<Feedback> {
        ScreenshotWait {
            state: self.0.clone(),
            request_id,
        }
        .await
    }

    pub fn close(&self) {
        close(&self.0);
    }
}

struct ScreenshotWait {
    state: Rc<RefCell<State>>,
    request_id: u64,
}

impl Future for ScreenshotWait {
    type Output = Result<Feedback>;

    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Poll::Ready(Err("Viewer closed while awaiting screenshot".into()));
        }
        if let Some(index) = state.feedback.iter().position(|event| {
            matches!(event,
            Feedback::ScreenshotTaken { request_id, .. } if *request_id == self.request_id)
        }) {
            state.waiters.remove(&self.request_id);
            return Poll::Ready(Ok(state.feedback.remove(index).unwrap()));
        }
        state.waiters.insert(self.request_id, cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for ScreenshotWait {
    fn drop(&mut self) {
        self.state.borrow_mut().waiters.remove(&self.request_id);
    }
}

impl RendererTransport {
    pub fn try_request(&self) -> Result<Option<Request>> {
        Ok(self.0.borrow_mut().requests.pop_front())
    }

    pub fn send_feedback(&self, feedback: Feedback) -> Result<()> {
        if feedback == Feedback::Closed {
            close(&self.0);
            return Ok(());
        }
        let waiter = {
            let mut state = self.0.borrow_mut();
            if state.closed {
                return Err("Viewer is closed".into());
            }
            let waiter = match &feedback {
                Feedback::ScreenshotTaken { request_id, .. } => state.waiters.remove(request_id),
                _ => None,
            };
            state.feedback.push_back(feedback);
            waiter
        };
        if let Some(waiter) = waiter {
            waiter.wake();
        }
        Ok(())
    }

    pub fn attach_repaint(&self, ctx: Context) {
        let pending = {
            let mut state = self.0.borrow_mut();
            state.repaint = Some(ctx.clone());
            !state.requests.is_empty()
        };
        if pending {
            ctx.request_repaint();
        }
    }
}

impl Drop for RendererTransport {
    fn drop(&mut self) {
        close(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::Wake,
    };

    fn screenshot(id: u64) -> Feedback {
        Feedback::ScreenshotTaken {
            request_id: id,
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        }
    }

    struct WakeCount(AtomicUsize);
    impl Wake for WakeCount {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn cloned_clients_share_fifo_requests() {
        let (client, renderer) = pair();
        client
            .send_request(Request::TakeScreenshot { request_id: 1 })
            .unwrap();
        client
            .clone()
            .send_request(Request::TakeScreenshot { request_id: 2 })
            .unwrap();
        assert!(matches!(
            renderer.try_request().unwrap(),
            Some(Request::TakeScreenshot { request_id: 1 })
        ));
        assert!(matches!(
            renderer.try_request().unwrap(),
            Some(Request::TakeScreenshot { request_id: 2 })
        ));
        assert!(renderer.try_request().unwrap().is_none());
    }

    #[test]
    fn concurrent_waiters_keep_other_screenshot_replies() {
        let (client, renderer) = pair();
        renderer.send_feedback(screenshot(2)).unwrap();
        renderer.send_feedback(screenshot(1)).unwrap();
        let mut ctx = TaskContext::from_waker(Waker::noop());
        let mut first = std::pin::pin!(client.wait_screenshot(1));
        let mut second = std::pin::pin!(client.wait_screenshot(2));
        assert_eq!(
            first.as_mut().poll(&mut ctx),
            Poll::Ready(Ok(screenshot(1)))
        );
        assert_eq!(
            second.as_mut().poll(&mut ctx),
            Poll::Ready(Ok(screenshot(2)))
        );
    }

    #[test]
    fn feedback_and_renderer_drop_wake_pending_waiters() {
        let (client, renderer) = pair();
        let count = Arc::new(WakeCount(AtomicUsize::new(0)));
        let waker = Waker::from(count.clone());
        let mut ctx = TaskContext::from_waker(&waker);
        let mut first = std::pin::pin!(client.wait_screenshot(1));
        assert!(first.as_mut().poll(&mut ctx).is_pending());
        renderer.send_feedback(screenshot(1)).unwrap();
        assert_eq!(count.0.load(Ordering::Relaxed), 1);
        assert!(first.as_mut().poll(&mut ctx).is_ready());
        let mut second = std::pin::pin!(client.wait_screenshot(2));
        assert!(second.as_mut().poll(&mut ctx).is_pending());
        drop(renderer);
        assert_eq!(count.0.load(Ordering::Relaxed), 2);
        assert!(matches!(
            second.as_mut().poll(&mut ctx),
            Poll::Ready(Err(_))
        ));
        assert!(
            client
                .send_request(Request::TakeScreenshot { request_id: 3 })
                .is_err()
        );
    }

    #[test]
    fn cancelled_waiter_is_unregistered() {
        let (client, _renderer) = pair();
        let mut ctx = TaskContext::from_waker(Waker::noop());
        let mut pending = Box::pin(client.wait_screenshot(1));
        assert!(pending.as_mut().poll(&mut ctx).is_pending());
        assert_eq!(client.0.borrow().waiters.len(), 1);
        drop(pending);
        assert!(client.0.borrow().waiters.is_empty());
    }

    #[test]
    fn explicit_close_wakes_all_cloned_client_waiters() {
        let (client, renderer) = pair();
        let clone = client.clone();
        let count = Arc::new(WakeCount(AtomicUsize::new(0)));
        let waker = Waker::from(count.clone());
        let mut ctx = TaskContext::from_waker(&waker);
        let mut first = std::pin::pin!(client.wait_screenshot(1));
        let mut second = std::pin::pin!(clone.wait_screenshot(2));
        assert!(first.as_mut().poll(&mut ctx).is_pending());
        assert!(second.as_mut().poll(&mut ctx).is_pending());
        clone.close();
        clone.close(); // Closing is idempotent.
        assert_eq!(count.0.load(Ordering::Relaxed), 2);
        assert!(matches!(first.as_mut().poll(&mut ctx), Poll::Ready(Err(_))));
        assert!(matches!(
            second.as_mut().poll(&mut ctx),
            Poll::Ready(Err(_))
        ));
        assert!(renderer.try_request().unwrap().is_none());
        assert!(client.try_feedback().is_err());
    }

    #[test]
    fn requests_queued_before_renderer_attachment_trigger_repaint() {
        let (client, renderer) = pair();
        let ctx = Context::default();
        for _ in 0..3 {
            let _ = ctx.run_ui(Default::default(), |_| {});
        }
        let count = Arc::new(AtomicUsize::new(0));
        let callback_count = count.clone();
        ctx.set_request_repaint_callback(move |_| {
            callback_count.fetch_add(1, Ordering::Relaxed);
        });
        client
            .send_request(Request::TakeScreenshot { request_id: 1 })
            .unwrap();
        assert_eq!(count.load(Ordering::Relaxed), 0);
        renderer.attach_repaint(ctx.clone());
        assert!(count.load(Ordering::Relaxed) > 0);
        assert!(matches!(
            renderer.try_request().unwrap(),
            Some(Request::TakeScreenshot { request_id: 1 })
        ));
    }
}
