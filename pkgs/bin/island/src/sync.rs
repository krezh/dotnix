use std::sync::{Mutex, MutexGuard, PoisonError};

/// Recovers a lock a panicking worker held, so what it guarded keeps being served.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
