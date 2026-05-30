//! Async / await / parallel / worker / threadPool runtime support.
//!
//! Real OS-thread concurrency (ADR-042). `async(thunk)` spawns a `std::thread` that runs the
//! thunk inside a fault-isolation boundary (`fault::with_async_boundary`); a runtime fault
//! becomes an `Error` value surfaced at `await` rather than aborting the process. The thunk's
//! captured env is deep-copied (Option C) so the worker owns a private graph and the parent's
//! non-atomic refcounts are never touched concurrently. The result is a boxed `TaggedVal*`
//! computed by the worker and handed to the parent through the promise (the worker thread has
//! exited by the time `await` reads it — ownership transfer, no shared access).

use crate::memory::lin_alloc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

/// A raw pointer asserted safe to move across threads. We uphold the invariant manually: the
/// only pointers wrapped here are (a) a deep-copied, thread-private closure env, and (b) a
/// transferable result the producing thread no longer touches after publishing it.
#[derive(Clone, Copy)]
struct SendPtr(*mut u8);
unsafe impl Send for SendPtr {}

/// The resolved state of a promise: either a value (boxed `TaggedVal*`, owned by the promise)
/// or an error message captured at the thread boundary (built into an `Error` object lazily at
/// `await`, to keep object construction on the awaiting thread).
enum PromiseState {
    Pending,
    Resolved(SendPtr),
    Failed(String),
}

/// A Promise — a future value computed on another OS thread (spec §32.2). Backed by a
/// mutex+condvar so `await` blocks until the worker publishes a result.
#[repr(C)]
pub struct LinPromise {
    inner: Arc<(Mutex<PromiseState>, Condvar)>,
    handle: Option<JoinHandle<()>>,
}

/// A ThreadPool (Phase 4). Placeholder layout retained for ABI compatibility until Phase 4
/// fills it in; `n` records the requested worker count.
#[repr(C)]
pub struct LinThreadPool {
    pub n: i32,
}

/// A Worker (Phase 5). Retained synchronous shape until Phase 5 replaces it with a real
/// long-lived thread + mailbox.
#[repr(C)]
pub struct LinWorker {
    pub on_msg_fn: *mut u8,
    pub on_msg_env: *mut u8,
    pub on_msg_has_env: u8,
}

/// Build an `Error` object `{ "type": "error", "message": <msg> }` as a boxed `TaggedVal*`.
/// Mirrors `http::make_error_object`. Runs on the awaiting thread.
unsafe fn make_error_tagged(msg: &str) -> *mut u8 {
    use crate::object::{lin_object_alloc, lin_object_set};
    use crate::string::lin_string_from_bytes;
    use crate::tagged::{alloc_tagged, TaggedVal, TAG_OBJECT, TAG_STR};
    let obj = lin_object_alloc(2);
    let mk = |s: &str| lin_string_from_bytes(s.as_ptr(), s.len() as u32);
    let type_tv = TaggedVal { tag: TAG_STR, _pad: [0; 7], payload: mk("error") as u64 };
    lin_object_set(obj, mk("type"), &type_tv);
    let msg_tv = TaggedVal { tag: TAG_STR, _pad: [0; 7], payload: mk(msg) as u64 };
    lin_object_set(obj, mk("message"), &msg_tv);
    alloc_tagged(TAG_OBJECT, obj as u64)
}

/// Allocate a LinPromise that is already resolved to `value` (no thread). Used for the
/// degenerate inline path and by combinators that need a settled promise.
#[no_mangle]
pub unsafe extern "C" fn lin_make_promise(value: *mut u8) -> *mut LinPromise {
    let inner = Arc::new((Mutex::new(PromiseState::Resolved(SendPtr(value))), Condvar::new()));
    let p = lin_alloc(std::mem::size_of::<LinPromise>()) as *mut LinPromise;
    std::ptr::write(p, LinPromise { inner, handle: None });
    p
}

/// Spawn the thunk closure `thunk` (a `*LinClosure` { rc, _pad, fn_ptr, env_ptr, .. }) on a
/// new OS thread and return a `LinPromise` for its result. The thunk's captured env is
/// deep-copied (Option C, ADR-042) so the worker owns a private graph and the parent's
/// non-atomic refcounts are never touched concurrently.
///
/// If the env is NOT transferable (no capture descriptor, or it captures a function/iterator —
/// `CAP_OPAQUE`), we cannot safely hand it to another thread, so the thunk is run **inline** on
/// the calling thread and the promise resolves immediately. This is the sound fallback: still
/// correct, just without parallelism for that one thunk. (The checker already bans `var`
/// captures and non-transferable *returns*; an opaque *captured function* is the only case that
/// reaches here, and §32.2.1 allows it — running inline keeps it correct.)
#[no_mangle]
pub unsafe extern "C" fn lin_async_spawn(thunk: *mut u8) -> *mut LinPromise {
    crate::fault::install_quiet_fault_hook();
    if thunk.is_null() {
        return lin_make_promise(std::ptr::null_mut());
    }
    // Closure layout: offset 8 = fn_ptr, offset 16 = env_ptr.
    let fn_ptr = *(thunk.add(8) as *const *mut u8);
    let env_ptr = *(thunk.add(16) as *const *mut u8);

    if !crate::transfer::env_is_transferable(env_ptr) {
        // Run inline (sound fallback). No fault boundary needed for parity with the previous
        // inline behaviour — actually we DO want fault isolation even inline, so wrap it.
        let outcome = crate::fault::with_async_boundary(|| {
            let call: unsafe extern "C-unwind" fn(*mut u8) -> *mut u8 = std::mem::transmute(fn_ptr);
            call(env_ptr)
        });
        return match outcome {
            Ok(v) => lin_make_promise(v),
            Err(msg) => lin_make_promise(make_error_tagged(&msg)),
        };
    }

    // Deep-copy the env so the worker owns a private graph (null env → null, no copy needed).
    let env_copy = crate::transfer::transfer_clone_env(env_ptr);
    // env_size for releasing the copy on the worker: 8 + count*8, recovered from the descriptor.
    let env_size: u64 = if env_copy.is_null() {
        0
    } else {
        let desc = *(env_copy as *const *const u8);
        let count = *(desc as *const u32) as u64;
        8 + count * 8
    };

    let inner = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
    let inner_for_thread = Arc::clone(&inner);
    // Capture the pointers as usize (unconditionally Send) and recast inside; the safety
    // invariant — env_copy is a thread-private deep copy, fn_ptr is read-only code — is
    // upheld manually (ADR-042 Option C).
    let fn_addr = fn_ptr as usize;
    let env_addr = env_copy as usize;

    let handle = std::thread::spawn(move || {
        let fn_ptr = fn_addr as *mut u8;
        let env_ptr = env_addr as *mut u8;
        let outcome = crate::fault::with_async_boundary(|| {
            let call: unsafe extern "C-unwind" fn(*mut u8) -> *mut u8 = std::mem::transmute(fn_ptr);
            call(env_ptr)
        });
        // Free the thread-private env copy now that the thunk has finished with it.
        if !env_ptr.is_null() && env_size > 0 {
            crate::transfer::release_env_copy(env_ptr, env_size);
        }
        let (lock, cvar) = &*inner_for_thread;
        let mut state = lock.lock().unwrap();
        *state = match outcome {
            Ok(v) => PromiseState::Resolved(SendPtr(v)),
            Err(msg) => PromiseState::Failed(msg),
        };
        cvar.notify_all();
    });

    let p = lin_alloc(std::mem::size_of::<LinPromise>()) as *mut LinPromise;
    std::ptr::write(p, LinPromise { inner, handle: Some(handle) });
    p
}

/// Await a promise: block until the worker publishes a result, join the thread, and return
/// the resolved `TaggedVal*` (or a freshly-built `Error` object on fault). Ownership of the
/// result transfers to the caller. Null promise → null (the Json null value).
#[no_mangle]
pub unsafe extern "C" fn lin_await_promise(promise: *mut LinPromise) -> *mut u8 {
    if promise.is_null() {
        return std::ptr::null_mut();
    }
    // Take the join handle so we can join exactly once.
    let handle = (*promise).handle.take();
    let inner = Arc::clone(&(*promise).inner);
    // Wait for resolution.
    let result = {
        let (lock, cvar) = &*inner;
        let mut state = lock.lock().unwrap();
        loop {
            match &*state {
                PromiseState::Pending => {
                    state = cvar.wait(state).unwrap();
                }
                PromiseState::Resolved(p) => break Ok(p.0),
                PromiseState::Failed(msg) => break Err(msg.clone()),
            }
        }
    };
    // Join the worker thread (it has already published; this reaps it).
    if let Some(h) = handle {
        let _ = h.join();
    }
    match result {
        Ok(v) => v,
        Err(msg) => make_error_tagged(&msg),
    }
}

/// Allocate a LinThreadPool with `n` workers. (Phase 4 fills in real worker threads.)
#[no_mangle]
pub unsafe extern "C" fn lin_thread_pool_new(n: i32) -> *mut LinThreadPool {
    let ptr = lin_alloc(std::mem::size_of::<LinThreadPool>()) as *mut LinThreadPool;
    (*ptr).n = n;
    ptr
}

/// Allocate a LinWorker with the given on_message closure. (Phase 5 replaces this.)
#[no_mangle]
pub unsafe extern "C" fn lin_worker_new(
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
    has_env: u8,
) -> *mut LinWorker {
    let ptr = lin_alloc(std::mem::size_of::<LinWorker>()) as *mut LinWorker;
    (*ptr).on_msg_fn = fn_ptr;
    (*ptr).on_msg_env = env_ptr;
    (*ptr).on_msg_has_env = has_env;
    ptr
}

/// Send a message to a worker and synchronously get the reply. (Phase 5 makes this a real
/// mailbox round-trip.)
#[no_mangle]
pub unsafe extern "C" fn lin_worker_request(worker: *mut LinWorker, msg: *mut u8) -> *mut u8 {
    if worker.is_null() {
        return std::ptr::null_mut();
    }
    let fn_ptr = (*worker).on_msg_fn;
    let env_ptr = (*worker).on_msg_env;
    let has_env = (*worker).on_msg_has_env;
    if fn_ptr.is_null() {
        return std::ptr::null_mut();
    }
    if has_env != 0 {
        let call: unsafe extern "C-unwind" fn(*mut u8, *mut u8) -> *mut u8 = std::mem::transmute(fn_ptr);
        call(env_ptr, msg)
    } else {
        let call: unsafe extern "C-unwind" fn(*mut u8) -> *mut u8 = std::mem::transmute(fn_ptr);
        call(msg)
    }
}

/// Fire-and-forget message to a worker (result discarded).
#[no_mangle]
pub unsafe extern "C" fn lin_worker_message(worker: *mut LinWorker, msg: *mut u8) {
    lin_worker_request(worker, msg);
}

/// Close a worker (no-op until Phase 5).
#[no_mangle]
pub unsafe extern "C" fn lin_worker_close(_worker: *mut LinWorker) {}

/// Call a plain (capture-less) thunk on a thread pool. Phase 4 enqueues; for now spawn inline
/// via a tiny synthetic closure shell so `lin_async_spawn` can read fn/env from offsets 8/16.
#[no_mangle]
pub unsafe extern "C" fn lin_pool_async_plain(
    _pool: *mut LinThreadPool,
    fn_ptr: *mut u8,
) -> *mut LinPromise {
    let shell = lin_alloc(24);
    *(shell.add(8) as *mut *mut u8) = fn_ptr;
    *(shell.add(16) as *mut *mut u8) = std::ptr::null_mut();
    let p = lin_async_spawn(shell);
    std::alloc::dealloc(shell, std::alloc::Layout::from_size_align_unchecked(24, 8));
    p
}

/// Call a closure thunk on a thread pool. Phase 4 enqueues; for now delegate to the spawn path
/// (which deep-copies the env). The `thunk` here is the full closure pointer.
#[no_mangle]
pub unsafe extern "C" fn lin_pool_async_closure(
    _pool: *mut LinThreadPool,
    thunk: *mut u8,
) -> *mut LinPromise {
    lin_async_spawn(thunk)
}
