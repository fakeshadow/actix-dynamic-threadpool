use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;

use std::sync::{mpsc::Receiver, Arc, Mutex};

use crate::builder::Builder;
use crate::ThreadPoolError;

pub(crate) struct ThreadPoolInner {
    name: Option<String>,

    // a MPSC channel receiver for storing jobs.
    rx: Mutex<Receiver<Job>>,

    // 2 counters packed in one usize. With the following bit flag:
    // { work: 0, active_threads: 0 }
    // active_threads would be in a range from 0 - max_threads.
    // work_count would start from 0 and inc/dec with the job channel's send/recv
    counter: AtomicUsize,

    // one work unit.
    one_work: usize,

    stack_size: Option<usize>,
    recv_timeout: Option<Duration>,
    min_threads: usize,
    max_threads: usize,
}

pub(crate) type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPoolInner {
    pub(crate) fn new(builder: Builder, rx: Receiver<Job>) -> Self {
        let max_threads = builder.max_threads.unwrap_or(8);

        let one_work = (max_threads + 1).next_power_of_two();

        let recv_timeout = if builder.idle_timeout == Duration::from_secs(0) {
            None
        } else {
            Some(builder.idle_timeout)
        };

        ThreadPoolInner {
            name: builder.thread_name.map(|name| format!("{}-worker", name)),
            rx: Mutex::new(rx),
            counter: AtomicUsize::new(0),
            one_work,
            stack_size: builder.thread_stack_size,
            recv_timeout,
            min_threads: builder.min_threads,
            max_threads,
        }
    }

    pub(crate) fn recv(&self) -> Result<Job, ThreadPoolError> {
        let rx = self.rx.lock().unwrap();

        Ok(match self.recv_timeout {
            Some(dur) => rx.recv_timeout(dur).map(|job| {
                self.dec_work_count();
                job
            })?,
            None => rx.recv().map(|job| {
                self.dec_work_count();
                job
            })?,
        })
    }

    // spawn new thread and return true if we successfully did that.
    pub(crate) fn spawn_thread(&self, inner: &Arc<Self>) -> bool {
        let did_inc = self.inc_active_thread();

        if did_inc {
            let mut builder = std::thread::Builder::new();
            if let Some(name) = self.name() {
                builder = builder.name(name.into());
            }
            if let Some(stack_size) = self.stack_size() {
                builder = builder.stack_size(stack_size);
            }

            let mut worker = Worker::from(inner.clone());

            builder.spawn(move || worker.handle_job()).unwrap();
        }

        did_inc
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(crate) fn min_idle_count(&self) -> usize {
        self.min_threads
    }

    pub(crate) fn stack_size(&self) -> Option<usize> {
        self.stack_size
    }

    // increment thread count and return false if we already hit max pool size.
    pub(crate) fn inc_active_thread(&self) -> bool {
        let mut counter = self.counter.load(Ordering::Relaxed);

        loop {
            if self.active_count(counter) == self.max_threads {
                return false;
            }

            match self.counter.compare_exchange_weak(
                counter,
                counter + 1,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(a) => counter = a,
            }
        }
    }

    // return true if we should spawn new with this decrement.
    pub(crate) fn dec_active_thread(&self) -> bool {
        let counter = self.counter.fetch_sub(1, Ordering::SeqCst);

        let active = self.active_count(counter) - 1;

        active < self.min_threads
    }

    // return true if we are able to drop the worker without going below min_idle.
    pub(crate) fn can_drop_idle(&self) -> bool {
        let counter = self.counter.load(Ordering::SeqCst);
        let active = self.active_count(counter);

        active > self.min_threads
    }

    // return if we should spawn new thread.
    pub(crate) fn inc_work_count(&self) -> bool {
        let counter = self.counter.fetch_add(self.one_work, Ordering::Relaxed);

        let active = self.active_count(counter);
        let work = self.work_count(counter);

        work > 0 && active < self.max_threads
    }

    fn dec_work_count(&self) {
        self.counter.fetch_sub(self.one_work, Ordering::Relaxed);
    }

    pub(crate) fn thread_count(&self) -> (usize, usize) {
        let counter = self.counter.load(Ordering::SeqCst);

        (self.active_count(counter), self.max_threads)
    }
}

impl ThreadPoolInner {
    fn active_count(&self, counter: usize) -> usize {
        counter & (self.one_work - 1)
    }

    fn work_count(&self, counter: usize) -> usize {
        counter & !(self.one_work - 1)
    }
}

// A worker of the pool.
struct Worker {
    pool: Arc<ThreadPoolInner>,
    schedule_shutdown: bool,
}

impl Worker {
    fn handle_job(&mut self) {
        loop {
            match self.pool.recv() {
                Ok(job) => {
                    job();
                }
                Err(e) => match e {
                    ThreadPoolError::Disconnect => {
                        self.schedule_shutdown = true;
                        break;
                    }
                    ThreadPoolError::TimeOut => {
                        if self.pool.can_drop_idle() {
                            self.schedule_shutdown = true;
                            break;
                        } else {
                            std::thread::yield_now();
                        }
                    }
                },
            }
        }
    }
}

impl From<Arc<ThreadPoolInner>> for Worker {
    fn from(pool: Arc<ThreadPoolInner>) -> Self {
        Self {
            pool,
            schedule_shutdown: false,
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let inner = &self.pool;

        let should_spawn = inner.dec_active_thread();

        if should_spawn && !self.schedule_shutdown {
            inner.spawn_thread(inner);
        }
    }
}
