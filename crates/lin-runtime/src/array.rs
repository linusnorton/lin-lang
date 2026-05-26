use std::alloc::{alloc, dealloc, realloc, Layout};

/// Heap-allocated growable array.
/// Layout: refcount (u32) | len (u64) | cap (u64) | data (*mut LinArrayElem)
/// Each element is a tagged { tag: u8, pad: [u8;7], payload: u64 } cell.
#[repr(C)]
pub struct LinArray {
    pub refcount: u32,
    _pad: u32,
    pub len: u64,
    pub cap: u64,
    pub data: *mut LinArrayElem,
}

#[repr(C)]
pub struct LinArrayElem {
    pub tag: u8,
    _pad: [u8; 7],
    /// For scalar types this is the value directly (int/float/bool/null).
    /// For pointer types (String, Array, Object, Closure) this is the pointer.
    pub payload: u64,
}

unsafe fn array_elem_layout(cap: u64) -> Layout {
    Layout::from_size_align_unchecked(
        std::mem::size_of::<LinArrayElem>() * cap as usize,
        std::mem::align_of::<LinArrayElem>(),
    )
}

unsafe fn array_layout() -> Layout {
    Layout::from_size_align_unchecked(
        std::mem::size_of::<LinArray>(),
        std::mem::align_of::<LinArray>(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn lin_array_alloc(initial_cap: u64) -> *mut LinArray {
    let cap = initial_cap.max(4);
    let arr_layout = array_layout();
    let ptr = alloc(arr_layout) as *mut LinArray;
    (*ptr).refcount = 1;
    (*ptr).len = 0;
    (*ptr).cap = cap;
    let elem_layout = array_elem_layout(cap);
    (*ptr).data = alloc(elem_layout) as *mut LinArrayElem;
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn lin_array_free(arr: *mut LinArray) {
    let cap = (*arr).cap;
    dealloc((*arr).data as *mut u8, array_elem_layout(cap));
    dealloc(arr as *mut u8, array_layout());
}

/// Decrement refcount and free if zero (does not recurse into elements).
#[no_mangle]
pub unsafe extern "C" fn lin_array_release(arr: *mut LinArray) {
    if arr.is_null() {
        return;
    }
    (*arr).refcount -= 1;
    if (*arr).refcount == 0 {
        lin_array_free(arr);
    }
}

/// Push an element. `elem_ptr` points to the value; `tag` is the type tag.
#[no_mangle]
pub unsafe extern "C" fn lin_array_push(arr: *mut LinArray, elem_ptr: *const u8, tag: u8) {
    let len = (*arr).len;
    let cap = (*arr).cap;
    if len == cap {
        let new_cap = cap * 2;
        let old_layout = array_elem_layout(cap);
        let new_layout = array_elem_layout(new_cap);
        (*arr).data = realloc((*arr).data as *mut u8, old_layout, new_layout.size()) as *mut LinArrayElem;
        (*arr).cap = new_cap;
    }
    let slot = (*arr).data.add(len as usize);
    (*slot).tag = tag;
    // Copy 8 bytes from elem_ptr into payload (assumes elem fits in 8 bytes).
    std::ptr::copy_nonoverlapping(elem_ptr, &mut (*slot).payload as *mut u64 as *mut u8, 8);
    (*arr).len = len + 1;
}

/// Push an element that is already a TaggedVal* (copies tag+payload inline).
/// Avoids double indirection: the element is stored inline, not as a pointer to TaggedVal.
#[no_mangle]
pub unsafe extern "C" fn lin_array_push_tagged(arr: *mut LinArray, tagged: *const u8) {
    let len = (*arr).len;
    let cap = (*arr).cap;
    if len == cap {
        let new_cap = cap * 2;
        let old_layout = array_elem_layout(cap);
        let new_layout = array_elem_layout(new_cap);
        (*arr).data = realloc((*arr).data as *mut u8, old_layout, new_layout.size()) as *mut LinArrayElem;
        (*arr).cap = new_cap;
    }
    let slot = (*arr).data.add(len as usize);
    // Copy 16 bytes (full TaggedVal = LinArrayElem) from tagged into slot.
    std::ptr::copy_nonoverlapping(tagged, slot as *mut u8, 16);
    (*arr).len = len + 1;
}

/// Get a pointer to the element payload at index. Panics (exits) on OOB.
#[no_mangle]
pub unsafe extern "C" fn lin_array_get(arr: *const LinArray, idx: i64) -> *mut LinArrayElem {
    let len = (*arr).len as i64;
    if idx < 0 || idx >= len {
        eprintln!("Runtime error: array index {} out of bounds (len {})", idx, len);
        std::process::exit(1);
    }
    (*arr).data.add(idx as usize)
}

#[no_mangle]
pub unsafe extern "C" fn lin_array_length(arr: *const LinArray) -> i64 {
    (*arr).len as i64
}

// -------------------------------------------------------------------------
// Flat (unboxed) scalar arrays
// -------------------------------------------------------------------------
//
// When the element type is a known scalar (i32, i64, f32, f64) the codegen
// emits calls to these functions instead of the tagged LinArrayElem variants.
// Layout: same header as LinArray, but `data` points to raw T-sized elements.
// We reuse the LinArray struct — the `data` pointer just stores T* cast to
// *mut LinArrayElem.  A flat i32 array stores 4-byte elements; the tag byte
// is never written.
//
// Flat array: refcount | _pad | len | cap | data(*mut T)
// The `data` field is typed as *mut LinArrayElem for layout compatibility but
// treated as *mut T internally — always accessed via the flat functions below.

// --- i32 ---

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_alloc_i32(initial_cap: u64) -> *mut LinArray {
    let cap = initial_cap.max(4);
    let arr_layout = array_layout();
    let ptr = alloc(arr_layout) as *mut LinArray;
    (*ptr).refcount = 1;
    (*ptr).len = 0;
    (*ptr).cap = cap;
    let data_layout = Layout::from_size_align_unchecked(
        std::mem::size_of::<i32>() * cap as usize,
        std::mem::align_of::<i32>(),
    );
    (*ptr).data = alloc(data_layout) as *mut LinArrayElem;
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_push_i32(arr: *mut LinArray, val: i32) {
    let len = (*arr).len;
    let cap = (*arr).cap;
    if len == cap {
        let new_cap = cap * 2;
        let old_layout = Layout::from_size_align_unchecked(
            std::mem::size_of::<i32>() * cap as usize,
            std::mem::align_of::<i32>(),
        );
        let new_size = std::mem::size_of::<i32>() * new_cap as usize;
        (*arr).data = realloc((*arr).data as *mut u8, old_layout, new_size) as *mut LinArrayElem;
        (*arr).cap = new_cap;
    }
    let data = (*arr).data as *mut i32;
    *data.add(len as usize) = val;
    (*arr).len = len + 1;
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_get_i32(arr: *const LinArray, idx: i64) -> i32 {
    let len = (*arr).len as i64;
    if idx < 0 || idx >= len {
        eprintln!("Runtime error: array index {} out of bounds (len {})", idx, len);
        std::process::exit(1);
    }
    let data = (*arr).data as *const i32;
    *data.add(idx as usize)
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_free_i32(arr: *mut LinArray) {
    let layout = Layout::from_size_align_unchecked(
        std::mem::size_of::<i32>() * (*arr).cap as usize,
        std::mem::align_of::<i32>(),
    );
    dealloc((*arr).data as *mut u8, layout);
    dealloc(arr as *mut u8, array_layout());
}

// --- i64 ---

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_alloc_i64(initial_cap: u64) -> *mut LinArray {
    let cap = initial_cap.max(4);
    let arr_layout = array_layout();
    let ptr = alloc(arr_layout) as *mut LinArray;
    (*ptr).refcount = 1;
    (*ptr).len = 0;
    (*ptr).cap = cap;
    let data_layout = Layout::from_size_align_unchecked(
        std::mem::size_of::<i64>() * cap as usize,
        std::mem::align_of::<i64>(),
    );
    (*ptr).data = alloc(data_layout) as *mut LinArrayElem;
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_push_i64(arr: *mut LinArray, val: i64) {
    let len = (*arr).len;
    let cap = (*arr).cap;
    if len == cap {
        let new_cap = cap * 2;
        let old_layout = Layout::from_size_align_unchecked(
            std::mem::size_of::<i64>() * cap as usize,
            std::mem::align_of::<i64>(),
        );
        let new_size = std::mem::size_of::<i64>() * new_cap as usize;
        (*arr).data = realloc((*arr).data as *mut u8, old_layout, new_size) as *mut LinArrayElem;
        (*arr).cap = new_cap;
    }
    let data = (*arr).data as *mut i64;
    *data.add(len as usize) = val;
    (*arr).len = len + 1;
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_get_i64(arr: *const LinArray, idx: i64) -> i64 {
    let len = (*arr).len as i64;
    if idx < 0 || idx >= len {
        eprintln!("Runtime error: array index {} out of bounds (len {})", idx, len);
        std::process::exit(1);
    }
    let data = (*arr).data as *const i64;
    *data.add(idx as usize)
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_free_i64(arr: *mut LinArray) {
    let layout = Layout::from_size_align_unchecked(
        std::mem::size_of::<i64>() * (*arr).cap as usize,
        std::mem::align_of::<i64>(),
    );
    dealloc((*arr).data as *mut u8, layout);
    dealloc(arr as *mut u8, array_layout());
}

// --- f32 ---

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_alloc_f32(initial_cap: u64) -> *mut LinArray {
    let cap = initial_cap.max(4);
    let arr_layout = array_layout();
    let ptr = alloc(arr_layout) as *mut LinArray;
    (*ptr).refcount = 1;
    (*ptr).len = 0;
    (*ptr).cap = cap;
    let data_layout = Layout::from_size_align_unchecked(
        std::mem::size_of::<f32>() * cap as usize,
        std::mem::align_of::<f32>(),
    );
    (*ptr).data = alloc(data_layout) as *mut LinArrayElem;
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_push_f32(arr: *mut LinArray, val: f32) {
    let len = (*arr).len;
    let cap = (*arr).cap;
    if len == cap {
        let new_cap = cap * 2;
        let old_layout = Layout::from_size_align_unchecked(
            std::mem::size_of::<f32>() * cap as usize,
            std::mem::align_of::<f32>(),
        );
        let new_size = std::mem::size_of::<f32>() * new_cap as usize;
        (*arr).data = realloc((*arr).data as *mut u8, old_layout, new_size) as *mut LinArrayElem;
        (*arr).cap = new_cap;
    }
    let data = (*arr).data as *mut f32;
    *data.add(len as usize) = val;
    (*arr).len = len + 1;
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_get_f32(arr: *const LinArray, idx: i64) -> f32 {
    let len = (*arr).len as i64;
    if idx < 0 || idx >= len {
        eprintln!("Runtime error: array index {} out of bounds (len {})", idx, len);
        std::process::exit(1);
    }
    let data = (*arr).data as *const f32;
    *data.add(idx as usize)
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_free_f32(arr: *mut LinArray) {
    let layout = Layout::from_size_align_unchecked(
        std::mem::size_of::<f32>() * (*arr).cap as usize,
        std::mem::align_of::<f32>(),
    );
    dealloc((*arr).data as *mut u8, layout);
    dealloc(arr as *mut u8, array_layout());
}

// --- f64 ---

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_alloc_f64(initial_cap: u64) -> *mut LinArray {
    let cap = initial_cap.max(4);
    let arr_layout = array_layout();
    let ptr = alloc(arr_layout) as *mut LinArray;
    (*ptr).refcount = 1;
    (*ptr).len = 0;
    (*ptr).cap = cap;
    let data_layout = Layout::from_size_align_unchecked(
        std::mem::size_of::<f64>() * cap as usize,
        std::mem::align_of::<f64>(),
    );
    (*ptr).data = alloc(data_layout) as *mut LinArrayElem;
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_push_f64(arr: *mut LinArray, val: f64) {
    let len = (*arr).len;
    let cap = (*arr).cap;
    if len == cap {
        let new_cap = cap * 2;
        let old_layout = Layout::from_size_align_unchecked(
            std::mem::size_of::<f64>() * cap as usize,
            std::mem::align_of::<f64>(),
        );
        let new_size = std::mem::size_of::<f64>() * new_cap as usize;
        (*arr).data = realloc((*arr).data as *mut u8, old_layout, new_size) as *mut LinArrayElem;
        (*arr).cap = new_cap;
    }
    let data = (*arr).data as *mut f64;
    *data.add(len as usize) = val;
    (*arr).len = len + 1;
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_get_f64(arr: *const LinArray, idx: i64) -> f64 {
    let len = (*arr).len as i64;
    if idx < 0 || idx >= len {
        eprintln!("Runtime error: array index {} out of bounds (len {})", idx, len);
        std::process::exit(1);
    }
    let data = (*arr).data as *const f64;
    *data.add(idx as usize)
}

#[no_mangle]
pub unsafe extern "C" fn lin_flat_array_free_f64(arr: *mut LinArray) {
    let layout = Layout::from_size_align_unchecked(
        std::mem::size_of::<f64>() * (*arr).cap as usize,
        std::mem::align_of::<f64>(),
    );
    dealloc((*arr).data as *mut u8, layout);
    dealloc(arr as *mut u8, array_layout());
}
