//! Generic IO + threading scaffolding around blocking dialogs and file writes.
//! Product-specific messages, workers, and effects live under their owning
//! workspace.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use eframe::egui;

enum IoCompletion<M> {
    Message(M),
    WorkerFailed(WorkerFailure),
    Silent,
}

/// Product-blind description of a worker that unwound before producing its
/// normal terminal message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkerFailure {
    task_name: &'static str,
}

impl WorkerFailure {
    pub(crate) fn task_name(self) -> &'static str {
        self.task_name
    }

    pub(crate) fn message(self) -> &'static str {
        "Background operation failed unexpectedly. Please try again."
    }

    pub(crate) fn notice(self) -> String {
        format!("{} failed unexpectedly. Please try again.", self.task_name)
    }
}

pub(crate) enum IoEvent<M> {
    Message(M),
    WorkerFailed(WorkerFailure),
}

/// One product-owned worker queue.
///
/// The queue knows only its message type and terminal accounting. Workspace
/// routing, product payloads, and effect application stay with the caller.
pub(crate) struct IoQueue<M> {
    tx: Sender<IoCompletion<M>>,
    rx: Receiver<IoCompletion<M>>,
    in_flight: usize,
}

impl<M> Default for IoQueue<M> {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx,
            in_flight: 0,
        }
    }
}

impl<M> IoQueue<M> {
    fn begin(&mut self) -> Sender<IoCompletion<M>> {
        self.in_flight += 1;
        self.tx.clone()
    }

    /// Drain completed messages and release one in-flight slot for every
    /// terminal signal, including cancellation and a caught worker panic.
    pub(crate) fn drain_events(&mut self) -> Vec<IoEvent<M>> {
        let mut events = Vec::new();
        while let Ok(completion) = self.rx.try_recv() {
            debug_assert!(
                self.in_flight > 0,
                "IO completion received without a matching task"
            );
            // Saturate rather than panic in release: a stray completion must
            // not take the UI thread down, and an idle queue is the safe state.
            self.in_flight = self.in_flight.saturating_sub(1);
            match completion {
                IoCompletion::Message(message) => events.push(IoEvent::Message(message)),
                IoCompletion::WorkerFailed(failure) => {
                    events.push(IoEvent::WorkerFailed(failure));
                }
                IoCompletion::Silent => {}
            }
        }
        events
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.in_flight > 0
    }

    pub(crate) fn is_idle(&self) -> bool {
        !self.is_busy()
    }
}

/// The basename of `path` as a `String`, or `""` when there is no file name.
/// Shared by the workspace ingest workers that label parsed files.
pub(crate) fn file_name_lossy(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

/// The basename shown after a successful save, falling back to the complete
/// display path when no Unicode basename is available.
pub(crate) fn saved_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

/// Run blocking IO on a worker thread and post exactly one terminal signal.
///
/// `None` is a silent terminal result (normally a dismissed picker). A panic is
/// caught, logged with `task_name`, and posts a typed worker failure so the
/// owning workspace can persist the failure and never remain busy forever.
pub(crate) fn spawn_io<M, F>(
    ctx: &egui::Context,
    queue: &mut IoQueue<M>,
    task_name: &'static str,
    work: F,
) where
    M: Send + 'static,
    F: FnOnce() -> Option<M> + Send + 'static,
{
    spawn_io_with_panic_handler(ctx, queue, task_name, work, |_| None);
}

fn spawn_io_with_panic_handler<M, F, R>(
    ctx: &egui::Context,
    queue: &mut IoQueue<M>,
    task_name: &'static str,
    work: F,
    recover: R,
) where
    M: Send + 'static,
    F: FnOnce() -> Option<M> + Send + 'static,
    R: FnOnce(WorkerFailure) -> Option<M> + Send + 'static,
{
    let tx = queue.begin();
    let ctx = ctx.clone();
    thread::spawn(move || {
        let completion = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
            Ok(Some(message)) => IoCompletion::Message(message),
            Ok(None) => IoCompletion::Silent,
            Err(_) => {
                tracing::error!("IO worker panicked during {task_name}; releasing in-flight gate");
                let failure = WorkerFailure { task_name };
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| recover(failure))) {
                    Ok(Some(message)) => IoCompletion::Message(message),
                    Ok(None) | Err(_) => IoCompletion::WorkerFailed(failure),
                }
            }
        };
        let _ = tx.send(completion);
        ctx.request_repaint();
    });
}

/// Shared save-CSV worker: rfd save dialog (csv filter) -> `fs::write` -> wrap
/// the outcome in the owning workspace's message constructor.
pub(crate) fn start_save_csv<M: Send + 'static>(
    ctx: &egui::Context,
    queue: &mut IoQueue<M>,
    bytes: Vec<u8>,
    default_name: &'static str,
    title: &'static str,
    msg: fn(Result<PathBuf, String>) -> M,
) {
    spawn_io(ctx, queue, title, move || {
        rfd::FileDialog::new()
            .add_filter("csv", &["csv"])
            .set_file_name(default_name)
            .set_title(title)
            .save_file()
            .map(|path| {
                let result = std::fs::write(&path, &bytes)
                    .map(|_| path)
                    .map_err(|e| e.to_string());
                msg(result)
            })
    });
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn panicked_io_worker_posts_terminal_message() {
        let ctx = egui::Context::default();
        let mut queue = IoQueue::<()>::default();
        spawn_io(&ctx, &mut queue, "synthetic test", || -> Option<()> {
            panic!("synthetic worker panic")
        });

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut events = Vec::new();
        while queue.is_busy() && Instant::now() < deadline {
            events.extend(queue.drain_events());
            thread::sleep(Duration::from_millis(5));
        }

        assert!(queue.is_idle());
        assert!(matches!(
            events.as_slice(),
            [IoEvent::WorkerFailed(WorkerFailure {
                task_name: "synthetic test"
            })]
        ));
    }

    #[test]
    fn cancelled_io_worker_releases_its_slot_without_a_product_message() {
        let ctx = egui::Context::default();
        let mut queue = IoQueue::<()>::default();
        spawn_io(&ctx, &mut queue, "synthetic cancellation", || None);

        let deadline = Instant::now() + Duration::from_secs(2);
        while queue.is_busy() && Instant::now() < deadline {
            assert!(queue.drain_events().is_empty());
            thread::sleep(Duration::from_millis(5));
        }

        assert!(queue.is_idle());
    }

    #[test]
    fn panicked_io_worker_can_post_a_product_recovery_message() {
        let ctx = egui::Context::default();
        let mut queue = IoQueue::<u8>::default();
        spawn_io_with_panic_handler(
            &ctx,
            &mut queue,
            "owned synthetic test",
            || -> Option<u8> { panic!("synthetic owned worker panic") },
            |_| Some(42),
        );

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut events = Vec::new();
        while queue.is_busy() && Instant::now() < deadline {
            events.extend(queue.drain_events());
            thread::sleep(Duration::from_millis(5));
        }

        assert!(queue.is_idle());
        assert!(matches!(events.as_slice(), [IoEvent::Message(42)]));
    }

    #[test]
    fn panicked_recovery_falls_back_to_a_worker_failure() {
        let ctx = egui::Context::default();
        let mut queue = IoQueue::<()>::default();
        spawn_io_with_panic_handler(
            &ctx,
            &mut queue,
            "broken recovery test",
            || -> Option<()> { panic!("synthetic owned worker panic") },
            |_| -> Option<()> { panic!("synthetic recovery panic") },
        );

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut events = Vec::new();
        while queue.is_busy() && Instant::now() < deadline {
            events.extend(queue.drain_events());
            thread::sleep(Duration::from_millis(5));
        }

        assert!(queue.is_idle());
        assert!(matches!(
            events.as_slice(),
            [IoEvent::WorkerFailed(WorkerFailure {
                task_name: "broken recovery test"
            })]
        ));
    }
}
