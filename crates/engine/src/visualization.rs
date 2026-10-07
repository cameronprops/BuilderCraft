//! Native latest-value scene publisher. One worker and one pending drawing per session.
use buildercraft_kernel::{Cancellation, Id};
use cadcraft_doc::Drawing;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::Duration,
};

struct Job {
    drawing: Arc<Drawing>,
    revision: u64,
    cancel: Cancellation,
}
#[derive(Default)]
struct State {
    pending: Option<Job>,
    active: Option<Cancellation>,
    stopped: bool,
    sequence: Option<u64>,
    revision: Option<u64>,
    error: Option<String>,
}
pub(crate) struct Feed {
    pub uid: u64,
    shared: Arc<(Mutex<State>, Condvar)>,
    worker: Option<JoinHandle<()>>,
    last: Option<Arc<Drawing>>,
}
impl Feed {
    pub fn new(uid: u64, project: Id, directory: PathBuf) -> std::io::Result<Self> {
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let work = shared.clone();
        let worker = std::thread::Builder::new().name("buildercraft-scene-feed".into()).spawn(move || {
            let (mutex, wake) = &*work;
            loop {
                let Ok(mut state) = mutex.lock() else { return };
                while state.pending.is_none() && !state.stopped {
                    let Ok(next) = wake.wait(state) else { return };
                    state = next;
                }
                if state.stopped {
                    return;
                }
                // A short settling window replaces bursts with their latest drawing.
                let Ok((mut state, _)) = wake.wait_timeout(state, Duration::from_millis(100)) else { return };
                if state.stopped {
                    return;
                }
                let Some(job) = state.pending.take() else { continue };
                state.active = Some(job.cancel.clone());
                drop(state);
                let result = cadcraft_io::visualization::snapshot(&job.drawing, project, job.revision, Default::default(), &job.cancel)
                    .and_then(|snapshot| cadcraft_io::visualization::publish_cancellable(&directory, snapshot, &job.cancel));
                let Ok(mut state) = mutex.lock() else { return };
                state.active = None;
                if job.cancel.check().is_ok() {
                    match result {
                        Ok(sequence) => {
                            state.sequence = Some(sequence);
                            state.revision = Some(job.revision);
                            state.error = None;
                        }
                        Err(error) => state.error = Some(error.to_string()),
                    }
                }
            }
        })?;
        Ok(Self { uid, shared, worker: Some(worker), last: None })
    }
    pub fn submit(&mut self, drawing: Arc<Drawing>, revision: u64) {
        if self.last.as_ref().is_some_and(|last| Arc::ptr_eq(last, &drawing)) {
            return;
        }
        let (mutex, wake) = &*self.shared;
        let Ok(mut state) = mutex.lock() else { return };
        if state.stopped {
            return;
        }
        if let Some(active) = &state.active {
            active.cancel();
        }
        self.last = Some(drawing.clone());
        state.pending = Some(Job { drawing, revision, cancel: Cancellation::default() });
        wake.notify_one();
    }
    pub fn status(&self) -> Value {
        match self.shared.0.lock() {
            Ok(state) => {
                json!({"running": !state.stopped,"document_uid":self.uid,"pending":state.pending.is_some(),"busy":state.active.is_some(),"sequence":state.sequence.map(|n|n.to_string()),"source_revision":state.revision.map(|n|n.to_string()),"error":state.error})
            }
            Err(_) => json!({"running":false,"error":"publisher state unavailable"}),
        }
    }
}
impl Drop for Feed {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.0.lock() {
            state.stopped = true;
            state.pending = None;
            if let Some(active) = &state.active {
                active.cancel();
            }
            self.shared.1.notify_one();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
