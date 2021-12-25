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
const MAX_ACTIONS: usize = 64;

static STORE: Lazy<FrameTimestampStoreFacade> =
    Lazy::new(|| {
        let (sender, receiver) = tmpsc::channel(MAX_ACTIONS);
        let instance = Mutex::new(FrameTimestampStore::new(receiver));
        FrameTimestampStoreFacade { sender, instance }
    });

static CONTROLLER: Lazy<Mutex<LatencyController>> =
    Lazy::new(|| Mutex::new(LatencyController::new()));

struct FrameTimestampStoreFacade {
    sender: tmpsc::Sender<Action>,
    instance: Mutex<FrameTimestampStore>
}

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
    while let Err(TrySendError::Full(_)) = STORE.sender.try_send(action) {
        STORE.instance.lock().drop_oldest_action();
    }
    info!("queue {:?}", action);
}

pub fn reset() {
    STORE.instance.lock().reset();
    CONTROLLER.lock().reset();
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
    let timestamp = STORE.instance.lock().submit(frame_index);
    CONTROLLER.lock().submit(timestamp)
}

pub fn new_time_sync() -> TimeSync {
    CONTROLLER.lock().new_time_sync()
}

pub fn get_fec_failure_state() -> bool {
    CONTROLLER.lock().get_fec_failure_state()
}

pub fn fec_failure() {
    CONTROLLER.lock().fec_failure()
}

pub fn set_fec_failure_state(fec_failure: bool) {
    CONTROLLER.lock().set_fec_failure_state(fec_failure)
}

pub fn packet_loss(lost: u32) {
    CONTROLLER.lock().packet_loss(lost)
}

pub fn set_total_latency(latency: u32) {
    CONTROLLER.lock().set_total_latency(latency)
}

struct FrameTimestampStore {
    action_receiver: tmpsc::Receiver<Action>,
    frames: [FrameTimestamp; MAX_FRAMES],
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

impl FrameTimestampStore {
    fn frames_default() -> [FrameTimestamp; MAX_FRAMES] {
        [FrameTimestamp { ..Default::default() }; MAX_FRAMES]
    }

    fn new(action_receiver: tmpsc::Receiver<Action>) -> Self {
        Self {
            action_receiver,
            frames: FrameTimestampStore::frames_default()
        }
    }

    fn reset(&mut self) {
        while self.drop_oldest_action() {}
        self.frames = FrameTimestampStore::frames_default()
    }

    fn get_frame(&mut self, frame_index: u64) -> &mut FrameTimestamp {
        let frame = &mut self.frames[(frame_index as usize) % MAX_FRAMES];
        if frame.frame_index != frame_index {
            *frame = Default::default();
            frame.frame_index = frame_index;
        }
        frame
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

    fn submit(&mut self, frame_index: u64) -> FrameTimestamp {
        self.get_frame(frame_index).submit = util::get_timestamp_us();
        debug!("submit {} {}", frame_index, self.get_frame(frame_index).submit);

        while let Ok(action) = self.action_receiver.try_recv() {
            if action.frame_index < frame_index { continue; }
            match action.time {
                ActTime::Tracking(time) => {
                    self.tracking(action.frame_index, time);
                }
                ActTime::Received(sent_time, received_time) => {
                    self.received(action.frame_index, sent_time, received_time);
                }
                ActTime::EstimatedSent(estimated_sent_time, received_time) => {
                    self.estimated_sent(action.frame_index, estimated_sent_time, received_time);
                }
                ActTime::ReceivedFirst(time) => {
                    self.received_first(action.frame_index, time);
                }
                ActTime::ReceivedLast(time) => {
                    self.received_last(action.frame_index, time);
                }
                ActTime::DecoderInput(time) => {
                    self.decoder_input(action.frame_index, time);
                }
                ActTime::DecoderOutput(time) => {
                    self.decoder_output(action.frame_index, time);
                }
                ActTime::Rendered(time) => {
                    self.rendered1(action.frame_index, time);
                    self.rendered2(action.frame_index, time);
                }
            }
        }

        *self.get_frame(frame_index)
    }

    fn drop_oldest_action(&mut self) -> bool {
        return if let Ok(action) = self.action_receiver.try_recv() {
            warn!("drop {:?}", action);
            true
        } else {
            false
        }
    }
}

struct LatencyController {
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

impl LatencyController {
    fn new() -> LatencyController {
        LatencyController {
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

    fn submit(&mut self, timestamp: FrameTimestamp) -> bool {
        let mut invalid_timestamp = false;

        if timestamp.estimated_sent > timestamp.received_last {
            error!("invalid timestamp: {} estimated_sent {} > received_last {} ",
                   timestamp.frame_index, timestamp.estimated_sent, timestamp.received_last);
            invalid_timestamp = true;
        }
        if timestamp.decoder_input > timestamp.decoder_output {
            error!("invalid timestamp: {} decoder_input {} > decoder_output {}",
                   timestamp.frame_index, timestamp.decoder_input, timestamp.decoder_output);
            invalid_timestamp = true;
        }
        if timestamp.received != 0 && timestamp.tracking > timestamp.received {
            error!("invalid timestamp: {} tracking {} > received {}",
                   timestamp.frame_index, timestamp.tracking, timestamp.received);
            invalid_timestamp = true;
        }
        if self.last_submit >= timestamp.submit {
            error!("invalid timestamp: {} last_submit {} >= submit {}",
                   timestamp.frame_index, self.last_submit, timestamp.submit);
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