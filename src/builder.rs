use core::time::Duration;

use crate::pool::ThreadPool;

#[derive(Clone)]
pub struct Builder {
    pub(crate) min_threads: usize,
    pub(crate) max_threads: Option<usize>,
    pub(crate) thread_name: Option<String>,
    pub(crate) thread_stack_size: Option<usize>,
    pub(crate) idle_timeout: Duration,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            min_threads: 1,
            max_threads: None,
            thread_name: None,
            thread_stack_size: None,
            idle_timeout: Duration::from_secs(60 * 5),
        }
    }
}

impl Builder {
    /// Set the maximum number of worker-threads that will be alive at any given moment by the built
    /// [`ThreadPool`]. If not specified, defaults the number of threads to the number of CPUs.
    ///
    /// # Panics
    ///
    /// This method will panic if `max_threads` is 0.
    pub fn max_threads(mut self, num: usize) -> Builder {
        assert!(num > 0);
        self.max_threads = Some(num);
        self
    }

    /// Set the minimal number of worker-threads that will be alive at any given moment by the built
    /// [`ThreadPool`]. If not specified, defaults the number of threads to the number of 1.
    ///
    /// # Panics
    ///
    /// This method will panic if `min_threads` is 0.
    pub fn min_threads(mut self, num: usize) -> Builder {
        assert!(num > 0);
        self.min_threads = num;
        self
    }

    /// Set the thread name for each of the threads spawned by the built [`ThreadPool`]. If not
    /// specified, threads spawned by the thread pool will be unnamed.
    pub fn thread_name(mut self, name: impl Into<String>) -> Builder {
        self.thread_name = Some(name.into());
        self
    }

    // /// Set the stack size (in bytes) for each of the threads spawned by the built [`ThreadPool`].
    // /// If not specified, threads spawned by the threadpool will have a stack size [as specified in
    // /// the `std::thread` documentation][thread].
    // ///
    // /// [thread]: https://doc.rust-lang.org/nightly/std/thread/index.html#stack-size
    // pub fn thread_stack_size(mut self, size: usize) -> Builder {
    //     self.thread_stack_size = Some(size);
    //     self
    // }

    /// Set the idle time out for a single thread.
    ///
    /// The thread would de spawn itself when it kept in idle state for this Duration
    ///
    /// Default is 5 minutes.
    ///
    /// *. Pass Duration::from_secs(0) would bypass the timeout.
    pub fn idle_timeout(mut self, dur: Duration) -> Builder {
        self.idle_timeout = dur;
        self
    }

    /// Finalize the [`Builder`] and build the [`ThreadPool`].
    pub fn build(self) -> ThreadPool {
        self.into()
    }
}
