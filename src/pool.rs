use core::fmt::{Debug, Formatter, Result as FmtResult};

use std::sync::{
    mpsc::{channel, Sender},
    Arc,
};

use crate::builder::Builder;
use crate::pool_inner::{Job, ThreadPoolInner};

/// Abstraction of a thread pool for basic parallelism.
pub struct ThreadPool {
    tx: Sender<Job>,
    inner: Arc<ThreadPoolInner>,
}

impl ThreadPool {
    /// Creates a new thread pool builder
    ///
    /// See [`Builder`]
    pub fn builder() -> Builder {
        Default::default()
    }

    /// Executes the function `job` on a thread in the pool.
    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let inner = &self.inner;

        self.tx.send(Box::new(job)).unwrap();

        // read from the previous state when we increment work count
        if inner.inc_work_count() {
            inner.spawn_thread(inner);
        }
    }

    /// Return a state of the pool.
    pub fn state(&self) -> ThreadPoolState {
        let name = self.inner.name();
        let (active_threads, max_threads) = self.inner.thread_count();

        ThreadPoolState {
            name,
            active_threads,
            max_threads,
        }
    }
}

impl From<Builder> for ThreadPool {
    fn from(builder: Builder) -> Self {
        let (tx, rx) = channel();

        let inner = ThreadPoolInner::new(builder, rx);

        let pool = ThreadPool {
            tx,
            inner: Arc::new(inner),
        };

        let inner = &pool.inner;

        let min_idle = inner.min_idle_count();

        (0..min_idle).for_each(|_| {
            inner.spawn_thread(&inner);
        });

        pool
    }
}

impl Clone for ThreadPool {
    /// Cloning a pool will create a new instance to the pool.
    fn clone(&self) -> ThreadPool {
        ThreadPool {
            tx: self.tx.clone(),
            inner: self.inner.clone(),
        }
    }
}

pub struct ThreadPoolState<'a> {
    pub name: Option<&'a str>,
    pub active_threads: usize,
    pub max_threads: usize,
}

impl Debug for ThreadPool {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.state().fmt(f)
    }
}

impl Debug for ThreadPoolState<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ThreadPoolState")
            .field("name", &self.name)
            .field("active_threads", &self.active_threads)
            .field("max_threads", &self.max_threads)
            .finish()
    }
}
