use std::sync::mpsc::{RecvError, RecvTimeoutError};

/// An enum exposed pool internal error to public.
pub enum ThreadPoolError {
    TimeOut,
    Disconnect,
}

impl From<RecvError> for ThreadPoolError {
    fn from(_e: RecvError) -> Self {
        ThreadPoolError::Disconnect
    }
}

impl From<RecvTimeoutError> for ThreadPoolError {
    fn from(e: RecvTimeoutError) -> Self {
        match e {
            RecvTimeoutError::Disconnected => ThreadPoolError::Disconnect,
            RecvTimeoutError::Timeout => ThreadPoolError::TimeOut,
        }
    }
}
