/// FORMIX AO-native WASM Contract for HyperBEAM (~wasm64@1.0)
///
/// State lives in global Rust statics. HyperBEAM snapshots/restores the
/// entire WASM linear memory between messages, so globals survive across
/// invocations — unlike CWAO where state had to be read/written via
/// db_read/db_write on each message.
mod getrandom_impl;
mod handlers;
mod message;
mod state;

use std::cell::UnsafeCell;

use message::{AOMessage, AOResponse};
use state::ProcessState;

// ─── Global State ──────────────────────────────────────────────────────────────
// Safety: WASM is single-threaded; no concurrent access possible.
// UnsafeCell + manual Sync avoids Rust 2024's static_mut_refs prohibition.
struct SyncCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for SyncCell<T> {}

static PROCESS_STATE: SyncCell<Option<ProcessState>> = SyncCell(UnsafeCell::new(None));
static RESPONSE_BUF: SyncCell<Vec<u8>> = SyncCell(UnsafeCell::new(Vec::new()));

fn get_state() -> &'static mut ProcessState {
    unsafe {
        let state = &mut *PROCESS_STATE.0.get();
        if state.is_none() {
            *state = Some(ProcessState::new());
        }
        state.as_mut().unwrap()
    }
}

// ─── WASM Entry Point ──────────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub extern "C" fn handle(msg_ptr: i32, msg_len: i32) -> i32 {
    if msg_len <= 0 {
        let response = AOResponse::error("Invalid message length");
        let response_json = serde_json::to_vec(&response).unwrap_or_else(|_| b"{}".to_vec());
        return write_response(response_json);
    }

    let msg_bytes = unsafe { core::slice::from_raw_parts(msg_ptr as *const u8, msg_len as usize) };

    let response = match serde_json::from_slice::<AOMessage>(msg_bytes) {
        Ok(msg) => handlers::dispatch(get_state(), msg),
        Err(e) => AOResponse::error(format!("Failed to parse message: {e}")),
    };

    let response_json = serde_json::to_vec(&response).unwrap_or_else(|_| b"{}".to_vec());
    write_response(response_json)
}

// ─── Memory helpers ────────────────────────────────────────────────────────────
fn write_response(data: Vec<u8>) -> i32 {
    unsafe {
        let buf = &mut *RESPONSE_BUF.0.get();
        *buf = data;
        buf.as_ptr() as i32
    }
}

/// Host calls this to allocate WASM memory before writing the incoming message.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: i32) -> i32 {
    if size <= 0 {
        return 0;
    }
    let layout = std::alloc::Layout::from_size_align(size as usize, 1).expect("invalid layout");
    let ptr = unsafe { std::alloc::alloc(layout) };
    ptr as i32
}

/// Host calls this to free previously allocated WASM memory.
#[unsafe(no_mangle)]
pub extern "C" fn dealloc(ptr: i32, size: i32) {
    if ptr == 0 || size <= 0 {
        return;
    }
    let layout = std::alloc::Layout::from_size_align(size as usize, 1).expect("invalid layout");
    unsafe {
        std::alloc::dealloc(ptr as *mut u8, layout);
    }
}

/// Returns the length of the last response (so the host can read ptr..ptr+len).
#[unsafe(no_mangle)]
pub extern "C" fn response_len() -> i32 {
    unsafe {
        let buf = &*RESPONSE_BUF.0.get();
        buf.len() as i32
    }
}
