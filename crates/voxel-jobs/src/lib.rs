//! Jobs are generation-tagged: the main thread drops results from an old camera epoch.
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::{sync::Arc, thread};

pub struct Job<T> {
    pub epoch: u64,
    pub priority: i32,
    pub value: T,
}
pub struct Pool<T: Send + 'static, R: Send + 'static> {
    tx: Sender<Job<T>>,
    pub results: Receiver<(u64, R)>,
}

impl<T: Send + 'static, R: Send + 'static> Pool<T, R> {
    pub fn new(workers: usize, f: impl Fn(T) -> R + Send + Sync + 'static) -> Self {
        let (tx, rx) = unbounded::<Job<T>>();
        let (out_tx, results) = unbounded();
        let f = Arc::new(f);
        for _ in 0..workers.max(1) {
            let rx = rx.clone();
            let out_tx = out_tx.clone();
            let f = f.clone();
            thread::spawn(move || {
                while let Ok(job) = rx.recv() {
                    let _ = out_tx.send((job.epoch, f(job.value)));
                }
            });
        }
        Self { tx, results }
    }
    pub fn submit(&self, job: Job<T>) {
        let _ = self.tx.send(job);
    }
}
