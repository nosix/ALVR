use crate::{
    legacy_packets::TimeSync,
    util,
};
use alvr_common::prelude::*;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

const MAX_FRAMES: usize = 1024;

pub static INSTANCE: Lazy<Mutex<LatencyController>> =
    Lazy::new(|| Mutex::new(LatencyController::new()));

pub struct LatencyController {
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

    pub fn reset(&mut self) {
        *self = LatencyController::new();
    }

    pub fn estimated_sent(&mut self, frame_index: u64, offset: u64) {
        self.get_frame(frame_index).estimated_sent = util::get_timestamp_us() - offset;
        debug!("estimated_sent {}", self.get_frame(frame_index).estimated_sent);
    }

    pub fn received(&mut self, frame_index: u64, timestamp: u64) {
        self.get_frame(frame_index).received = timestamp;
        debug!("received {}", self.get_frame(frame_index).received);
    }

    pub fn received_first(&mut self, frame_index: u64) {
        self.get_frame(frame_index).received_first = util::get_timestamp_us();
        debug!("received_first {}", self.get_frame(frame_index).received_first);
    }

    // FIXME May be called multiple times with the same index
    pub fn received_last(&mut self, frame_index: u64) {
        self.get_frame(frame_index).received_last = util::get_timestamp_us();
        debug!("received_last {}", self.get_frame(frame_index).received_last);
    }

    // FIXME May be called multiple times with the same index
    pub fn decoder_input(&mut self, frame_index: u64) {
        self.get_frame(frame_index).decoder_input = util::get_timestamp_us();
        debug!("decoder_input {}", self.get_frame(frame_index).decoder_input);
    }

    // FIXME May be called multiple times with the same index
    pub fn decoder_output(&mut self, frame_index: u64) {
        self.get_frame(frame_index).decoder_output = util::get_timestamp_us();
        debug!("decoder_output {}", self.get_frame(frame_index).decoder_output);
    }

    pub fn rendered1(&mut self, frame_index: u64) {
        self.get_frame(frame_index).rendered1 = util::get_timestamp_us();
        debug!("rendered1 {}", self.get_frame(frame_index).rendered1);
    }

    pub fn rendered2(&mut self, frame_index: u64) {
        self.get_frame(frame_index).rendered2 = util::get_timestamp_us();
        debug!("rendered2 {}", self.get_frame(frame_index).rendered2);
    }

    pub fn tracking(&mut self, frame_index: u64) {
        self.get_frame(frame_index).tracking = util::get_timestamp_us();
        debug!("tracking {}", self.get_frame(frame_index).tracking);
    }

    pub fn submit(&mut self, frame_index: u64) -> bool {
        self.get_frame(frame_index).submit = util::get_timestamp_us();

        let timestamp = *self.get_frame(frame_index);

        if timestamp.estimated_sent > timestamp.received_last ||
            timestamp.decoder_input > timestamp.decoder_output ||
            timestamp.tracking > timestamp.received ||
            self.last_submit >= timestamp.submit {
            error!("invalid timestamp");
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
        self.last_submit = timestamp.submit;

        return true;
    }

    pub fn packet_loss(&mut self, lost: u32) {
        let lost = lost as u64;
        self.check_and_reset_second();
        self.packets_lost_total += lost;
        self.packets_lost_in_second += lost;
        debug!("packet_loss {} {}", self.packets_lost_total, self.packets_lost_in_second);
    }

    pub fn fec_failure(&mut self) {
        self.check_and_reset_second();
        self.fec_failure_total += 1;
        self.fec_failure_in_second += 1;
        debug!("fec_failure {} {}", self.fec_failure_total, self.fec_failure_in_second);
    }

    pub fn set_total_latency(&mut self, latency: u32) {
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

    pub fn get_fec_failure_state(&self) -> bool {
        self.fec_failure_state
    }

    pub fn set_fec_failure_state(&mut self, fec_failure: bool) {
        self.fec_failure_state = fec_failure;
    }

    fn increment_time_sync_sequence(&mut self) -> u64 {
        let time_sync_sequence = self.time_sync_sequence;
        self.time_sync_sequence =
            if time_sync_sequence == u64::MAX { 0 } else { time_sync_sequence + 1 };
        return time_sync_sequence;
    }

    pub fn new_time_sync(&mut self) -> TimeSync {
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