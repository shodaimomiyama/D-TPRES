/// FORMIX AO-native WASM Contract for HyperBEAM (~wasm64@1.0)
///
/// Architecture:
/// - State is managed as global Rust statics, persisted via WASM memory snapshots.
///   HyperBEAM serializes/deserializes the entire WASM memory between message calls.
/// - Entry point: `handle(msg_ptr: i32, msg_len: i32) -> i32`
///   HyperBEAM writes the message JSON to WASM memory, calls handle with ptr+len.
///   handle returns a pointer to the JSON-encoded result.
/// - No CosmWasm host functions. No db_read/db_write. Pure WASM.
///
/// Migration from ao_cwao/:
/// - cosmwasm_std / cw-storage-plus / cw-multi-test → removed
/// - instantiate/execute/query/reply → single `handle` function
/// - MAP.save(deps.storage, key, &data) → PROCESS_STATE.insert(key, data)
/// - SubMsg (cross-contract) → ao_send import (async AO message)
/// - umbral-pre logic: unchanged
/// - getrandom custom impl: unchanged

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use umbral_pre;

mod getrandom_impl;
mod message;
mod state;
mod handlers;

use message::{AOMessage, AOResponse};
use state::ProcessState;

// ─── Global State ──────────────────────────────────────────────────────────────
//
// In AO's execution model, WASM memory is serialized (snapshot) after each
// message and deserialized (restore) before the next. This means global statics
// survive across invocations as long as HyperBEAM manages the snapshot.
//
// TODO: Replace with a proper global once mutex requirements are clear for WASM
// (WASM is single-threaded, so static mut is safe here)
static mut PROCESS_STATE: Option<ProcessState> = None;

fn get_state() -> &'static mut ProcessState {
    // Safety: WASM is single-threaded
    unsafe {
        if PROCESS_STATE.is_none() {
            PROCESS_STATE = Some(ProcessState::new());
        }
        PROCESS_STATE.as_mut().unwrap()
    }
}

// ─── WASM Entry Point ──────────────────────────────────────────────────────────
//
// HyperBEAM dev_wasm calls this function via:
//   hb_beamr:call(Instance, "handle", [msg_ptr, msg_len], ...)
//
// Parameters are passed as WASM i32 values pointing into WASM linear memory.
// The host writes the message JSON to WASM memory before calling.
// The return value is a pointer to the JSON-encoded response in WASM memory.
#[no_mangle]
pub extern "C" fn handle(msg_ptr: i32, msg_len: i32) -> i32 {
    let msg_bytes = unsafe {
        std::slice::from_raw_parts(msg_ptr as *const u8, msg_len as usize)
    };

    let response = match serde_json::from_slice::<AOMessage>(msg_bytes) {
        Ok(msg) => handlers::dispatch(get_state(), msg),
        Err(e) => AOResponse::error(format!("Failed to parse message: {e}")),
    };

    let response_json = serde_json::to_vec(&response).unwrap_or_else(|_| b"{}".to_vec());

    // Write response to WASM memory and return pointer
    // The host will call the `read` function to retrieve the response by pointer
    write_response(response_json)
}

// ─── Memory helpers ────────────────────────────────────────────────────────────
//
// Static buffer for response output.
// NOTE: This is a placeholder implementation.
// Real implementation needs proper allocator + length-prefix or length return.
static mut RESPONSE_BUF: Vec<u8> = Vec::new();

fn write_response(data: Vec<u8>) -> i32 {
    // Safety: single-threaded WASM
    unsafe {
        RESPONSE_BUF = data;
        RESPONSE_BUF.as_ptr() as i32
    }
}

/// Allocator for host-side writes into WASM memory.
/// HyperBEAM uses hb_beamr_io::write_string which writes directly to WASM memory.
/// Exposing alloc allows the host to allocate space before writing.
#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let buf = vec![0u8; size as usize];
    let ptr = buf.as_ptr() as i32;
    std::mem::forget(buf); // prevent deallocation
    ptr
}

/// Returns the length of the last response written to RESPONSE_BUF.
/// The host reads: ptr = handle(...), len = response_len(), then reads ptr..ptr+len.
#[no_mangle]
pub extern "C" fn response_len() -> i32 {
    unsafe { RESPONSE_BUF.len() as i32 }
}
