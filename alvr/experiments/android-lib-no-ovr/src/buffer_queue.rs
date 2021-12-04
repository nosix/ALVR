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
    sync::Arc
};
use tokio::sync::mpsc as tmpsc;
use jni::descriptors::Desc;

static INPUT_BUFFER_SENDER: Lazy<Mutex<Option<tmpsc::UnboundedSender<InputBuffer>>>> =
    Lazy::new(|| Mutex::new(None));

static NAL_SENDER: Lazy<Mutex<Option<tmpsc::Sender<Nal>>>> =
    Lazy::new(|| Mutex::new(None));

static WAITING_INPUT_BUFFER: Lazy<Mutex<VecDeque<InputBuffer>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

const QUEUE_LIMIT: usize = 128;

pub fn push_input_buffer(buffer: InputBuffer) {
    if let Some(input_buffer_sender) = INPUT_BUFFER_SENDER.lock().as_ref() {
        input_buffer_sender.send(buffer);
    } else {
        WAITING_INPUT_BUFFER.lock().push_back(buffer);
    }
}

pub fn push_nal(nal: Nal) {
    NAL_SENDER.lock().as_ref().map(|nal_sender| {
        if let Err(e) = nal_sender.try_send(nal) {
            warn!("{} {}", e, trace_str!());
        }
    });
}

pub async fn buffer_coordination_loop(vm: Arc<JavaVM>) -> StrResult {
    let (input_buffer_sender, mut input_buffer_receiver) = tmpsc::unbounded_channel();
    let (nal_sender, mut nal_receiver) = tmpsc::channel(QUEUE_LIMIT);
    *INPUT_BUFFER_SENDER.lock() = Some(input_buffer_sender);
    *NAL_SENDER.lock() = Some(nal_sender);

    for waiting_buffer in WAITING_INPUT_BUFFER.lock().drain(..) {
        push_input_buffer(waiting_buffer);
    }

    loop {
        let input_buffer = input_buffer_receiver.recv().await
            .ok_or("InputBuffer can't be received.")?;
        let nal = nal_receiver.recv().await
            .ok_or("NAL can't be received.")?;

        if nal.nal_type == NalType::Sps {
            input_buffer.queue_config(&vm, nal);
        } else {
            input_buffer.queue(&vm, nal);
        }
    }
}