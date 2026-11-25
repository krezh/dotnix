//! Reads and frames pictures on a thread of its own, so the frame that first shows one, often the
//! first of a panel opening, is not held up by it.

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, mpsc},
    task::{Context, Poll},
};

use futures::{
    Stream,
    channel::mpsc::{UnboundedReceiver, unbounded},
};
use image::RgbaImage;

use super::paint::{Framing, framed_picture};
use crate::supervisor;

/// A picture at a size, in a framing, as the painter keeps it.
pub type Key = (String, u32, u32);

pub enum Source {
    File(PathBuf),
    /// Blue first.
    Pixels(Arc<RgbaImage>),
}

struct Job {
    key: Key,
    source: Source,
    framing: Framing,
}

pub struct Done {
    pub key: Key,
    /// Red first and premultiplied, its key's size; `None` if it could not be read.
    pub rgba: Option<Vec<u8>>,
    /// Held until taken in, so a pixel picture's place is not reused before then.
    _source: Source,
}

pub struct Decoder {
    jobs: Option<mpsc::Sender<Job>>,
    done: UnboundedReceiver<Done>,
    pending: HashSet<Key>,
}

impl Decoder {
    pub fn new() -> Self {
        let (jobs, asked) = mpsc::channel::<Job>();
        let (finished, done) = unbounded();
        supervisor::spawn("island-pictures", move || {
            for job in asked {
                let size = (job.key.1, job.key.2);
                let rgba = framed_picture(&job.source, size, job.framing);
                let done = Done {
                    key: job.key,
                    rgba,
                    _source: job.source,
                };
                if finished.unbounded_send(done).is_err()
                {
                    return;
                }
            }
        });
        Self {
            jobs: Some(jobs),
            done,
            pending: HashSet::new(),
        }
    }

    pub fn is_pending(&self, key: &Key) -> bool {
        self.pending.contains(key)
    }

    /// Sends it to be made, or gives it back to be made where it is wanted.
    pub fn ask(&mut self, key: &Key, source: Source, framing: Framing) -> Result<(), Source> {
        if self.pending.contains(key) {
            return Ok(());
        }
        let Some(jobs) = &self.jobs else {
            return Err(source);
        };
        let job = Job {
            key: key.clone(),
            source,
            framing,
        };
        if let Err(mpsc::SendError(job)) = jobs.send(job) {
            self.jobs = None;
            return Err(job.source);
        }
        self.pending.insert(key.clone());
        Ok(())
    }

    /// What has been made since last asked.
    pub fn finished(&mut self, cx: &mut Context<'_>) -> Vec<Done> {
        let mut done = Vec::new();
        while let Poll::Ready(Some(item)) = std::pin::Pin::new(&mut self.done).poll_next(cx) {
            self.pending.remove(&item.key);
            done.push(item);
        }
        done
    }
}
