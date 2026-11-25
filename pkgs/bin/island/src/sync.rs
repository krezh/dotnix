use std::sync::{
    Arc, Condvar, Mutex, MutexGuard, PoisonError,
    atomic::{AtomicUsize, Ordering},
};

/// Recovers a lock a panicking worker held, so what it guarded keeps being served.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How many hold a [`Watching`], such as islands showing a panel that a worker works harder for.
#[derive(Clone, Default)]
pub struct Watchers(Arc<Count>);

#[derive(Default)]
struct Count {
    held: AtomicUsize,
    lock: Mutex<()>,
    started: Condvar,
}

impl Watchers {
    /// Runs `on_first` when nothing was watching before.
    pub fn watch(&self, on_first: impl FnOnce()) -> Watching {
        if self.0.held.fetch_add(1, Ordering::SeqCst) == 0 {
            {
                // Held so a `wait` between its check and its sleep still hears this.
                let _held = lock(&self.0.lock);
                self.0.started.notify_all();
            }
            on_first();
        }
        Watching(self.clone())
    }

    pub fn watching(&self) -> bool {
        self.0.held.load(Ordering::SeqCst) > 0
    }

    /// Sleeps until something watches.
    pub fn wait(&self) {
        let guard = lock(&self.0.lock);
        drop(
            self.0
                .started
                .wait_while(guard, |_| !self.watching())
                .unwrap_or_else(PoisonError::into_inner),
        );
    }
}

/// Watches until dropped.
pub struct Watching(Watchers);

impl Drop for Watching {
    fn drop(&mut self) {
        self.0.0.held.fetch_sub(1, Ordering::SeqCst);
    }
}
