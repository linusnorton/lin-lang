/// Async/await/parallel/worker/threadPool runtime support.
/// All operations are implemented synchronously for simplicity:
/// async(thunk) eagerly evaluates the thunk and wraps the result in a promise.
/// This is semantically correct for non-concurrent programs.
use crate::memory::lin_alloc;

/// A Promise — holds a resolved TaggedVal*.
#[repr(C)]
pub struct LinPromise {
    pub value: *mut u8,
}

/// A ThreadPool — holds n (currently ignored, operates synchronously).
#[repr(C)]
pub struct LinThreadPool {
    pub n: i32,
}

/// A Worker — holds function pointers for on_message and on_shutdown.
/// Both are closure structs: { fn_ptr: ptr, env_ptr: ptr }.
#[repr(C)]
pub struct LinWorker {
    pub on_msg_fn: *mut u8,
    pub on_msg_env: *mut u8,
    pub on_msg_has_env: u8,
}

/// Allocate a LinPromise with a known TaggedVal* result.
/// Returns LinPromise*.
#[no_mangle]
pub unsafe extern "C" fn lin_make_promise(value: *mut u8) -> *mut LinPromise {
    let size = std::mem::size_of::<LinPromise>();
    let ptr = lin_alloc(size) as *mut LinPromise;
    (*ptr).value = value;
    ptr
}

/// Await a promise — returns its TaggedVal*.
#[no_mangle]
pub unsafe extern "C" fn lin_await_promise(promise: *mut LinPromise) -> *mut u8 {
    if promise.is_null() {
        return std::ptr::null_mut();
    }
    (*promise).value
}

/// Allocate a LinThreadPool with `n` workers.
#[no_mangle]
pub unsafe extern "C" fn lin_thread_pool_new(n: i32) -> *mut LinThreadPool {
    let size = std::mem::size_of::<LinThreadPool>();
    let ptr = lin_alloc(size) as *mut LinThreadPool;
    (*ptr).n = n;
    ptr
}

/// Allocate a LinWorker with the given on_message closure.
/// fn_ptr and env_ptr are from the closure struct; has_env = 1 if closure, 0 if plain fn ptr.
#[no_mangle]
pub unsafe extern "C" fn lin_worker_new(
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
    has_env: u8,
) -> *mut LinWorker {
    let size = std::mem::size_of::<LinWorker>();
    let ptr = lin_alloc(size) as *mut LinWorker;
    (*ptr).on_msg_fn = fn_ptr;
    (*ptr).on_msg_env = env_ptr;
    (*ptr).on_msg_has_env = has_env;
    ptr
}

/// Send a message to a worker and synchronously get the reply (TaggedVal*).
/// The on_message function is called with (env_ptr, msg) for closures, or (msg) for plain fns.
#[no_mangle]
pub unsafe extern "C" fn lin_worker_request(
    worker: *mut LinWorker,
    msg: *mut u8,
) -> *mut u8 {
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
        // Closure call: fn(env_ptr, msg) -> *mut u8
        let call: unsafe extern "C" fn(*mut u8, *mut u8) -> *mut u8 =
            std::mem::transmute(fn_ptr);
        call(env_ptr, msg)
    } else {
        // Plain function: fn(msg) -> *mut u8
        let call: unsafe extern "C" fn(*mut u8) -> *mut u8 =
            std::mem::transmute(fn_ptr);
        call(msg)
    }
}

/// Fire-and-forget message to a worker (result is discarded).
#[no_mangle]
pub unsafe extern "C" fn lin_worker_message(
    worker: *mut LinWorker,
    msg: *mut u8,
) {
    lin_worker_request(worker, msg);
}

/// Close a worker (no-op in synchronous implementation).
#[no_mangle]
pub unsafe extern "C" fn lin_worker_close(_worker: *mut LinWorker) {
    // Synchronous: nothing to shut down.
}

/// Call a thunk on a thread pool synchronously and return a promise.
/// pool: *mut LinThreadPool (ignored), thunk: fn_ptr or closure struct ptr.
/// Returns LinPromise*.
#[no_mangle]
pub unsafe extern "C" fn lin_pool_async_plain(
    _pool: *mut LinThreadPool,
    fn_ptr: *mut u8,
) -> *mut LinPromise {
    // Plain function ptr: call with 0 args.
    let call: unsafe extern "C" fn() -> *mut u8 = std::mem::transmute(fn_ptr);
    let result = call();
    lin_make_promise(result)
}

/// Call a closure thunk on a thread pool synchronously and return a promise.
#[no_mangle]
pub unsafe extern "C" fn lin_pool_async_closure(
    _pool: *mut LinThreadPool,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> *mut LinPromise {
    let call: unsafe extern "C" fn(*mut u8) -> *mut u8 = std::mem::transmute(fn_ptr);
    let result = call(env_ptr);
    lin_make_promise(result)
}
