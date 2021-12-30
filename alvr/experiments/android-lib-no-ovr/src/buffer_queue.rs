use crate::{
    jvm::InputBuffer,
    latency_controller,
    nal::{Nal, NalType},
};
use alvr_common::prelude::*;
use jni::JavaVM;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc as smpsc,
    },
};
use tokio::task;

static JAVA_VM: Lazy<Mutex<Option<JavaVM>>> =
    Lazy::new(|| Mutex::new(None));

static INPUT_BUFFER_SENDER: Lazy<Mutex<Option<smpsc::Sender<InputBuffer>>>> =
    Lazy::new(|| Mutex::new(None));

static NAL_SENDER: Lazy<Mutex<Option<smpsc::SyncSender<Nal>>>> =
    Lazy::new(|| Mutex::new(None));

static WAITING_INPUT_BUFFER: Lazy<Mutex<VecDeque<InputBuffer>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

static IDR_PARSED: AtomicBool = AtomicBool::new(false);

const QUEUE_LIMIT: usize = 128;

pub fn set_vm(vm: JavaVM) {
    *JAVA_VM.lock() = Some(vm);
}

pub fn is_idr_parsed() -> bool {
    IDR_PARSED.load(Ordering::Relaxed)
}

pub fn reset_idr_parsed() {
    IDR_PARSED.store(false, Ordering::Relaxed);
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

pub fn buffer_coordination_loop() -> task::JoinHandle<StrResult> {
    let (input_buffer_sender, input_buffer_receiver) = smpsc::channel();
    let (nal_sender, nal_receiver) = smpsc::sync_channel(QUEUE_LIMIT);
    *INPUT_BUFFER_SENDER.lock() = Some(input_buffer_sender);
    *NAL_SENDER.lock() = Some(nal_sender);

    // The main stream loop must be run in a normal thread, because it needs to access the JNI env
    // many times per second. If using a future I'm forced to attach and detach the env continuously.
    // When the parent function exits or gets canceled, this loop will run to finish.
    task::spawn_blocking(move || -> StrResult {
        let maybe_vm = JAVA_VM.lock();
        let maybe_env = if let Some(vm) = maybe_vm.as_ref() {
            Some(trace_err!(vm.attach_current_thread_permanently())?)
        } else {
            None
        };

        for waiting_buffer in WAITING_INPUT_BUFFER.lock().drain(..) {
            push_input_buffer(waiting_buffer)?;
        }

        if let Some(env) = maybe_env {
            loop {
                let input_buffer = trace_err!(input_buffer_receiver.recv())?;
                let nal = trace_err!(nal_receiver.recv())?;

                if nal.nal_type == NalType::Sps {
                    input_buffer.queue_config(&env, nal)?;
                } else {
                    if nal.nal_type == NalType::Idr {
                        IDR_PARSED.store(true, Ordering::Relaxed);
                    }
                    latency_controller::decoder_input(nal.frame_index);
                    input_buffer.queue(&env, nal)?;
                }
            }
        } else {
            warn!("JNIEnv has not been initialized. So, only the log is displayed.");
            loop {
                let nal = trace_err!(nal_receiver.recv())?;
                if nal.nal_type == NalType::Sps {
                    info!(
                        "queue_config {:?} frame_len={} frame_index={}",
                        nal.nal_type, nal.frame_buffer.len(), nal.frame_index
                    );
                } else {
                    if nal.nal_type == NalType::Idr {
                        IDR_PARSED.store(true, Ordering::Relaxed);
                    }
                    latency_controller::decoder_input(nal.frame_index);
                    info!(
                        "queue {:?} frame_len={} frame_index={}",
                        nal.nal_type, nal.frame_buffer.len(), nal.frame_index
                    );
                }
            }
        }
    })
}

pub fn terminate_loop() {
    *INPUT_BUFFER_SENDER.lock() = None;
    *NAL_SENDER.lock() = None;
    WAITING_INPUT_BUFFER.lock().clear();
    IDR_PARSED.store(false, Ordering::Relaxed);
    info!("terminate buffer_queue");
}