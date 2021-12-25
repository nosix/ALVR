use crate::{
    legacy_packets::TimeSync,
    util,
};
use alvr_common::prelude::*;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tokio::sync::mpsc::{
    self as tmpsc,
    error::TrySendError,
};

const MAX_FRAMES: usize = 1024;

static INSTANCE: Lazy<Mutex<LatencyController>> =
    Lazy::new(|| Mutex::new(LatencyController::new()));

static ACTION_CHANNEL: Lazy<ActionChannel> =
    Lazy::new(|| {
        let (sender, receiver) = tmpsc::channel(256);
        ActionChannel { sender, receiver: Mutex::new(receiver) }
    });

struct ActionChannel {
    sender: tmpsc::Sender<Action>,
    receiver: Mutex<tmpsc::Receiver<Action>>,
}

unsafe impl Sync for ActionChannel {}

#[derive(Copy, Clone, Debug)]
struct Action {
    frame_index: u64,
    time: ActTime,
}

#[derive(Copy, Clone, Debug)]
enum ActTime {
    Tracking(u64),
    Received(u64, u64),
    EstimatedSent(u64, u64),
    ReceivedFirst(u64),
    ReceivedLast(u64),
    DecoderInput(u64),
    DecoderOutput(u64),
    Rendered(u64),
}

fn queue(frame_index: u64, time: ActTime) {
    let action = Action { frame_index, time };
    while let Err(TrySendError::Full(_)) = ACTION_CHANNEL.sender.try_send(action) {
        if let Ok(action) = ACTION_CHANNEL.receiver.lock().try_recv() {
            warn!("drop {:?}", action);
        }
    }
    info!("queue {:?}", action);
}

pub fn reset() {
    let mut receiver = ACTION_CHANNEL.receiver.lock();
    while let Ok(_) = receiver.try_recv() {} // clear actions
    INSTANCE.lock().reset();
}

/// Record client time when sending TrackingInfo packet
pub fn tracking(frame_index: u64) {
    queue(frame_index, ActTime::Tracking(util::get_timestamp_us()));
}

/// Record estimated client time when the server sent a mode 3 TimeSync packet
pub fn received(frame_index: u64, sent_time: u64) {
    let current_time = util::get_timestamp_us();
    queue(frame_index, ActTime::Received(sent_time, current_time));
}

/// Record estimated client time when the server sent first video frame
pub fn estimated_sent(frame_index: u64, estimated_sent_time: u64) {
    let current_time = util::get_timestamp_us();
    queue(frame_index, ActTime::EstimatedSent(estimated_sent_time, current_time));
}

/// Record client time when received first video frame
pub fn received_first(frame_index: u64) {
    queue(frame_index, ActTime::ReceivedFirst(util::get_timestamp_us()));
}

// FIXME May be called multiple times with the same index
/// Record client time when received last video frame and pushing NAL
pub fn received_last(frame_index: u64) {
    queue(frame_index, ActTime::ReceivedLast(util::get_timestamp_us()));
}

// FIXME May be called multiple times with the same index
/// Record client time when IDR or P frame is queued to Decoder
pub fn decoder_input(frame_index: u64) {
    queue(frame_index, ActTime::DecoderInput(util::get_timestamp_us()));
}

// FIXME May be called multiple times with the same index
/// Record client time when the Decoder's output buffer becomes available
pub fn decoder_output(frame_index: u64) {
    queue(frame_index, ActTime::DecoderOutput(util::get_timestamp_us()));
}

/// Record client time when rendering is completed
pub fn rendered(frame_index: u64) {
    queue(frame_index, ActTime::Rendered(util::get_timestamp_us()));
}

pub fn submit(frame_index: u64) -> bool {
    update_timestamp(frame_index);
    return INSTANCE.lock().submit(frame_index);
}

fn update_timestamp(last_frame_index: u64) {
    let mut instance = INSTANCE.lock();
    let mut receiver = ACTION_CHANNEL.receiver.lock();
    while let Ok(action) = receiver.try_recv() {
        if action.frame_index < last_frame_index { continue; }
        match action.time {
            ActTime::Tracking(time) => {
                instance.tracking(action.frame_index, time);
            }
            ActTime::Received(sent_time, received_time) => {
                instance.received(action.frame_index, sent_time, received_time);
            }
            ActTime::EstimatedSent(estimated_sent_time, received_time) => {
                instance.estimated_sent(action.frame_index, estimated_sent_time, received_time);
            }
            ActTime::ReceivedFirst(time) => {
                instance.received_first(action.frame_index, time);
            }
            ActTime::ReceivedLast(time) => {
                instance.received_last(action.frame_index, time);
            }
            ActTime::DecoderInput(time) => {
                instance.decoder_input(action.frame_index, time);
            }
            ActTime::DecoderOutput(time) => {
                instance.decoder_output(action.frame_index, time);
            }
            ActTime::Rendered(time) => {
                instance.rendered1(action.frame_index, time);
                instance.rendered2(action.frame_index, time);
            }
        }
    }
}

pub fn new_time_sync() -> TimeSync {
    INSTANCE.lock().new_time_sync()
}

pub fn get_fec_failure_state() -> bool {
    INSTANCE.lock().get_fec_failure_state()
}

pub fn fec_failure() {
    INSTANCE.lock().fec_failure()
}

pub fn set_fec_failure_state(fec_failure: bool) {
    INSTANCE.lock().set_fec_failure_state(fec_failure)
}

pub fn packet_loss(lost: u32) {
    INSTANCE.lock().packet_loss(lost)
}

pub fn set_total_latency(latency: u32) {
    INSTANCE.lock().set_total_latency(latency)
}

struct LatencyController {
    frames: [FrameTimestamp; MAX_FRAMES],

    statistics_time: u64,
    packets_lost_total: u64,
    packets_lost_in_second: u64,
    packets_lost_previous: u64,
    fec_failure_total: u64,
    fec_failure_in_second: u64,
    fec_failure_previous: u64,

    server_total_latency: u32,

    // Total/Transport/Decode latency
    // Total/Max/Min/Count
    latency: [u32; 4],

    last_submit: u64,
    frames_in_second: f32,

    time_sync_sequence: u64,
    fec_failure_state: bool,
}

#[derive(Default, Copy, Clone)]
struct FrameTimestamp {
    frame_index: u64,

    // Timestamp in microsecond
    tracking: u64,
    estimated_sent: u64,
    received: u64,
    received_first: u64,
    received_last: u64,
    decoder_input: u64,
    decoder_output: u64,
    rendered1: u64,
    rendered2: u64,
    submit: u64,
}

impl LatencyController {
    fn new() -> LatencyController {
        LatencyController {
            frames: [FrameTimestamp { ..Default::default() }; MAX_FRAMES],
            statistics_time: util::get_timestamp_us() / util::US_IN_SEC,
            packets_lost_total: 0,
            packets_lost_in_second: 0,
            packets_lost_previous: 0,
            fec_failure_total: 0,
            fec_failure_in_second: 0,
            fec_failure_previous: 0,
            server_total_latency: 0,
            latency: [0; 4],

            last_submit: 0,
            frames_in_second: 0.0,
            time_sync_sequence: 0,
            fec_failure_state: false,
        }
    }

    fn reset(&mut self) {
        *self = LatencyController::new();
    }

    fn tracking(&mut self, frame_index: u64, time: u64) {
        self.get_frame(frame_index).tracking = time;
        debug!("tracking {} {}", frame_index, self.get_frame(frame_index).tracking);
    }

    fn received(&mut self, frame_index: u64, sent_time: u64, received_time: u64) {
        let tracking = self.get_frame(frame_index).tracking;
        if tracking < sent_time && sent_time < received_time {
            self.get_frame(frame_index).received = sent_time
        } else {
            warn!("received: The sent time is not included in the proper period. {} < {} < {}",
                  tracking, sent_time, received_time);
        }
        debug!("received {} {}", frame_index, self.get_frame(frame_index).received);
    }

    fn estimated_sent(&mut self, frame_index: u64, estimated_sent_time: u64, received_time: u64) {
        self.get_frame(frame_index).estimated_sent = if estimated_sent_time < received_time {
            estimated_sent_time
        } else {
            warn!("estimated_sent: The sent time is later than the receive time. {} < {}",
                  received_time, estimated_sent_time);
            received_time
        };
        debug!("estimated_sent {} {}", frame_index, self.get_frame(frame_index).estimated_sent);
    }

    fn received_first(&mut self, frame_index: u64, time: u64) {
        self.get_frame(frame_index).received_first = time;
        debug!("received_first {} {}", frame_index, self.get_frame(frame_index).received_first);
    }

    fn received_last(&mut self, frame_index: u64, time: u64) {
        self.get_frame(frame_index).received_last = time;
        debug!("received_last {} {}", frame_index, self.get_frame(frame_index).received_last);
    }

    fn decoder_input(&mut self, frame_index: u64, time: u64) {
        self.get_frame(frame_index).decoder_input = time;
        debug!("decoder_input {} {}", frame_index, self.get_frame(frame_index).decoder_input);
    }

    fn decoder_output(&mut self, frame_index: u64, time: u64) {
        self.get_frame(frame_index).decoder_output = time;
        debug!("decoder_output {} {}", frame_index, self.get_frame(frame_index).decoder_output);
    }

    /// Record client time when rendering is completed
    fn rendered1(&mut self, frame_index: u64, time: u64) {
        self.get_frame(frame_index).rendered1 = time;
        debug!("rendered1 {} {}", frame_index, self.get_frame(frame_index).rendered1);
    }

    /// Currently the same as rendered1
    fn rendered2(&mut self, frame_index: u64, time: u64) {
        self.get_frame(frame_index).rendered2 = time;
        debug!("rendered2 {} {}", frame_index, self.get_frame(frame_index).rendered2);
    }

    fn submit(&mut self, frame_index: u64) -> bool {
        self.get_frame(frame_index).submit = util::get_timestamp_us();

        let timestamp = *self.get_frame(frame_index);

        let mut invalid_timestamp = false;

        if timestamp.estimated_sent > timestamp.received_last {
            error!("invalid timestamp: {} estimated_sent {} > received_last {} ",
                   frame_index, timestamp.estimated_sent, timestamp.received_last);
            invalid_timestamp = true;
        }
        if timestamp.decoder_input > timestamp.decoder_output {
            error!("invalid timestamp: {} decoder_input {} > decoder_output {}",
                   frame_index, timestamp.decoder_input, timestamp.decoder_output);
            invalid_timestamp = true;
        }
        if timestamp.received != 0 && timestamp.tracking > timestamp.received {
            error!("invalid timestamp: {} tracking {} > received {}",
                   frame_index, timestamp.tracking, timestamp.received);
            invalid_timestamp = true;
        }
        if self.last_submit >= timestamp.submit {
            error!("invalid timestamp: {} last_submit {} >= submit {}",
                   frame_index, self.last_submit, timestamp.submit);
            invalid_timestamp = true;
        }

        if invalid_timestamp {
            return false;
        }

        self.latency[0] = (timestamp.submit - timestamp.tracking) as u32;
        self.latency[1] = (timestamp.received_last - timestamp.estimated_sent) as u32;
        self.latency[2] = (timestamp.decoder_output - timestamp.decoder_input) as u32;
        if timestamp.received != 0 {
            self.latency[3] = (timestamp.received - timestamp.tracking) as u32;
        } else {
            self.latency[3] = self.latency[1];
        }

        self.submit_new_frame();

        self.frames_in_second = 1000000.0 / (timestamp.submit - self.last_submit) as f32;
        info!("fps: {} = {} - {}", self.frames_in_second, timestamp.submit, self.last_submit);
        self.last_submit = timestamp.submit;

        return true;
    }

    fn packet_loss(&mut self, lost: u32) {
        let lost = lost as u64;
        self.check_and_reset_second();
        self.packets_lost_total += lost;
        self.packets_lost_in_second += lost;
        debug!("packet_loss {} {}", self.packets_lost_total, self.packets_lost_in_second);
    }

    fn fec_failure(&mut self) {
        self.check_and_reset_second();
        self.fec_failure_total += 1;
        self.fec_failure_in_second += 1;
        debug!("fec_failure {} {}", self.fec_failure_total, self.fec_failure_in_second);
    }

    fn set_total_latency(&mut self, latency: u32) {
        if latency < 200000 {
            self.server_total_latency =
                ((latency as f32) * 0.05 + (self.server_total_latency as f32) * 0.95) as u32;
        }
        debug!("set_total_latency {}", self.server_total_latency);
    }

    fn get_frame(&mut self, frame_index: u64) -> &mut FrameTimestamp {
        let frame = &mut self.frames[(frame_index as usize) % MAX_FRAMES];
        if frame.frame_index != frame_index {
            *frame = Default::default();
            frame.frame_index = frame_index;
        }
        frame
    }

    fn submit_new_frame(&mut self) {
        self.check_and_reset_second();
    }

    fn check_and_reset_second(&mut self) {
        let current = util::get_timestamp_us() / util::US_IN_SEC;
        if self.statistics_time != current {
            self.statistics_time = current;
            self.reset_second();
        }
    }

    fn reset_second(&mut self) {
        self.packets_lost_previous = self.packets_lost_in_second;
        self.packets_lost_in_second = 0;
        self.fec_failure_previous = self.fec_failure_in_second;
        self.fec_failure_in_second = 0;
    }

    fn get_fec_failure_state(&self) -> bool {
        self.fec_failure_state
    }

    fn set_fec_failure_state(&mut self, fec_failure: bool) {
        self.fec_failure_state = fec_failure;
    }

    fn increment_time_sync_sequence(&mut self) -> u64 {
        let time_sync_sequence = self.time_sync_sequence;
        self.time_sync_sequence =
            if time_sync_sequence == u64::MAX { 0 } else { time_sync_sequence + 1 };
        return time_sync_sequence;
    }

    fn new_time_sync(&mut self) -> TimeSync {
        TimeSync {
            mode: 0,

            sequence: self.increment_time_sync_sequence(),
            client_time: util::get_timestamp_us(),

            packets_lost_total: self.packets_lost_total,
            packets_lost_in_second: self.packets_lost_in_second,

            average_total_latency: self.latency[0],
            average_send_latency: self.latency[3],
            average_transport_latency: self.latency[1],
            average_decode_latency: self.latency[2],

            fec_failure: if self.fec_failure_state { 1 } else { 0 },
            fec_failure_in_second: self.fec_failure_in_second,
            fec_failure_total: self.fec_failure_total,

            fps: self.frames_in_second,

            ..Default::default()
        }
    }
}