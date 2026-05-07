use libubox_sys as sys;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// RAII guard for the global libubox event loop.
///
/// This guard exists for **interop with libubox-using C libraries** (e.g.
/// ubus clients) that internally register their own fds/timeouts/processes
/// against `uloop` and need `uloop_init` running. If you want a Rust event
/// loop, reach for `tokio` or `async-std` — `libubox` does not currently
/// wrap `uloop_fd_add`, `uloop_timeout_set`, `uloop_process_add`, etc.
///
/// Any future C-callable trampoline that calls user Rust code MUST wrap
/// it in `catch_unwind` and abort/log on panic; unwinding across the
/// libubox C frame is undefined behaviour.
///
/// libubox's uloop state (epoll fd, signal handlers, `do_sigchld`) lives in
/// process-global C statics, so at most one `Uloop` may exist per process;
/// [`new`](Self::new) returns `AlreadyInitialized` otherwise. The guard is
/// `!Send` / `!Sync` by convention to discourage cross-thread use; libubox's
/// globals aren't actually kernel-thread-bound.
///
/// `mem::forget`-ing the guard leaves the init flag set forever — don't.
///
/// ```compile_fail
/// fn assert_send<T: Send>() {}
/// assert_send::<libubox::Uloop>();
/// ```
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<libubox::Uloop>();
/// ```
pub struct Uloop {
    _not_send_sync: PhantomData<*const ()>,
}

#[derive(Debug)]
pub enum UloopError {
    AlreadyInitialized,
    InitFailed(i32),
}

impl fmt::Display for UloopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UloopError::AlreadyInitialized => f.write_str("uloop already initialized"),
            UloopError::InitFailed(rc) => write!(f, "uloop_init failed: rc={rc}"),
        }
    }
}

impl std::error::Error for UloopError {}

impl Uloop {
    pub fn new() -> Result<Self, UloopError> {
        if INITIALIZED.swap(true, Ordering::AcqRel) {
            return Err(UloopError::AlreadyInitialized);
        }
        // SAFETY: libubox global state; serialized by INITIALIZED.
        let rc = unsafe { sys::uloop_init() };
        if rc != 0 {
            INITIALIZED.store(false, Ordering::Release);
            return Err(UloopError::InitFailed(rc));
        }
        Ok(Self {
            _not_send_sync: PhantomData,
        })
    }

    /// Run the loop until `uloop_end` is called from a callback or a
    /// `SIGINT` / `SIGTERM` / `SIGQUIT` interrupts it. Returns `0` on a clean
    /// exit, or the signal number that interrupted it.
    ///
    /// Callbacks are driven by C code (e.g. ubus client handlers) that
    /// registered themselves against the loop. There is no Rust API for
    /// adding fds/timeouts here on purpose — see the type-level docs.
    pub fn run(&mut self) -> i32 {
        // `uloop_run` is a `static inline` in uloop.h that delegates to
        // `uloop_run_timeout(-1)`; replicate that here so we don't need
        // bindgen's `wrap_static_fns`.
        // SAFETY: holding `Self` proves uloop_init succeeded and uloop_done has not run.
        unsafe { sys::uloop_run_timeout(-1) }
    }
}

impl Drop for Uloop {
    fn drop(&mut self) {
        // SAFETY: matched with successful uloop_init in new().
        unsafe {
            sys::uloop_done();
        }
        INITIALIZED.store(false, Ordering::Release);
    }
}
