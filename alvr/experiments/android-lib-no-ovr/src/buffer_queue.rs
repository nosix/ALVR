use crate::{
    jvm::InputBuffer,
    legacy_packets::AlvrCodec,
    nal::{Nal, NalType},
};
use alvr_common::prelude::*;
use bytes::Bytes;
use jni::JavaVM;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc as smpsc,
    },
};
use tokio::task;

static INPUT_BUFFER_SENDER: Lazy<Mutex<Option<smpsc::Sender<InputBuffer>>>> =
    Lazy::new(|| Mutex::new(None));

static NAL_SENDER: Lazy<Mutex<Option<smpsc::SyncSender<Nal>>>> =
    Lazy::new(|| Mutex::new(None));

static WAITING_INPUT_BUFFER: Lazy<Mutex<VecDeque<InputBuffer>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

static IDR_PARSED: AtomicBool = AtomicBool::new(false);

const QUEUE_LIMIT: usize = 128;

pub fn is_idr_parsed() -> bool {
    IDR_PARSED.load(Ordering::Relaxed)
}

pub fn push_input_buffer(buffer: InputBuffer) -> StrResult {
    if let Some(input_buffer_sender) = INPUT_BUFFER_SENDER.lock().as_ref() {
        trace_err!(input_buffer_sender.send(buffer))?;
    } else {
        WAITING_INPUT_BUFFER.lock().push_back(buffer);
    }
    Ok(())
}

pub fn push_nal(nal: Nal) {
    NAL_SENDER.lock().as_ref().map(|nal_sender| {
        if let Err(e) = nal_sender.try_send(nal) {
            warn!("{} {}", e, trace_str!());
        }
    });
}

pub fn buffer_coordination_loop(vm: Arc<JavaVM>) -> task::JoinHandle<StrResult> {
    let (input_buffer_sender, mut input_buffer_receiver) = smpsc::channel();
    let (nal_sender, mut nal_receiver) = smpsc::sync_channel(QUEUE_LIMIT);
    *INPUT_BUFFER_SENDER.lock() = Some(input_buffer_sender);
    *NAL_SENDER.lock() = Some(nal_sender);

    // The main stream loop must be run in a normal thread, because it needs to access the JNI env
    // many times per second. If using a future I'm forced to attach and detach the env continuously.
    // When the parent function exits or gets canceled, this loop will run to finish.
    task::spawn_blocking(move || -> StrResult {
        let env = trace_err!(vm.attach_current_thread_permanently())?;

        for waiting_buffer in WAITING_INPUT_BUFFER.lock().drain(..) {
            push_input_buffer(waiting_buffer)?;
        }

        loop {
            let input_buffer = trace_err!(input_buffer_receiver.recv())?;
            let nal = trace_err!(nal_receiver.recv())?;

            if nal.nal_type == NalType::Sps {
                input_buffer.queue_config(&env, nal)?;
            } else {
                if nal.nal_type == NalType::Idr {
                    IDR_PARSED.store(true, Ordering::Relaxed);
                }
                input_buffer.queue(&env, nal)?;
            }
        }
    })
}