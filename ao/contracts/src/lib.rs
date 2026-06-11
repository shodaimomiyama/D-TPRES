// In test builds the `handle` export is compiled out, which makes the whole
// dispatch chain (handlers, state, entry plumbing) unreachable — allow it
// there so -D warnings only bites in production targets.
#![cfg_attr(test, allow(dead_code, unused_imports))]

//! FORMIX AO-native WASM Contract for HyperBEAM (JSON-Iface)
//!
//! Entry point ABI: handle(msg_ptr, env_ptr) -> *const u8
//! Memory exports: malloc(size) -> *mut u8, free(ptr)
//!
//! State lives in global Rust statics. HyperBEAM replays all messages
//! from genesis within a single WASM instance per compute request,
//! so globals accumulate correctly (O(n) cost, acceptable for PoC).

mod getrandom_impl;
mod handlers;
mod message;
mod state;

use std::cell::UnsafeCell;

use message::{AOIncomingMessage, AOResponse};
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

// ─── WASM Entry Point (JSON-Iface ABI) ──────────────────────────────────────
/// Called by JSON-Iface with two null-terminated JSON strings:
/// - msg_ptr: AO message (Tags format)
/// - env_ptr: Process definition (unused in PoC)
///
/// Returns pointer to null-terminated AOS-compatible JSON response.
///
/// # Safety
/// JSON-Iface guarantees msg_ptr is valid and null-terminated.
#[cfg(not(test))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn handle(msg_ptr: *const u8, _env_ptr: *const u8) -> *const u8 {
    if msg_ptr.is_null() {
        let response = AOResponse::error("Null message pointer");
        return write_aos_response(response);
    }

    let msg_bytes = unsafe { read_cstr(msg_ptr) };

    if msg_bytes.is_empty() {
        let response = AOResponse::error("Empty message");
        return write_aos_response(response);
    }

    let response = match serde_json::from_slice::<AOIncomingMessage>(msg_bytes) {
        Ok(incoming) => {
            let msg = incoming.into_ao_message();
            handlers::dispatch(get_state(), msg)
        }
        Err(e) => AOResponse::error(format!("Failed to parse message: {e}")),
    };

    write_aos_response(response)
}

// ─── Memory helpers ────────────────────────────────────────────────────────────
/// Read a null-terminated C string from a WASM memory pointer.
///
/// # Safety
/// Caller must ensure ptr is valid and points to a null-terminated string.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
unsafe fn read_cstr(ptr: *const u8) -> &'static [u8] {
    let mut len = 0;
    while unsafe { *ptr.add(len) != 0 } {
        len += 1;
    }
    unsafe { core::slice::from_raw_parts(ptr, len) }
}

/// Convert AOResponse to AOS format, serialize, null-terminate, and store.
fn write_aos_response(response: AOResponse) -> *const u8 {
    let aos = response.into_aos_response();
    let mut json = serde_json::to_vec(&aos)
        .unwrap_or_else(|_| br#"{"ok":false,"error":"Internal serialization error"}"#.to_vec());
    json.push(0);
    write_response(json)
}

fn write_response(data: Vec<u8>) -> *const u8 {
    unsafe {
        let buf = &mut *RESPONSE_BUF.0.get();
        *buf = data;
        buf.as_ptr()
    }
}

/// JSON-Iface calls this to allocate WASM memory for input strings.
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr
}

/// JSON-Iface calls this to free the response pointer.
/// PoC: no-op. HyperBEAM replays from genesis per compute request,
/// so accumulated leaks are bounded by message count.
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn free(_ptr: *mut u8) {}
