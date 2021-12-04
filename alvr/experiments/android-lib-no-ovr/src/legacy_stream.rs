use crate::{
    latency_controller,
    legacy_packets::*,
    nal::{Nal, NalParser},
    util,
};
use alvr_common::prelude::*;
use bytes::{Bytes, Buf};
use std::{sync::Arc, mem};

pub struct StreamHandler<F> where F : Fn(Nal) {
    time_diff: i64,
    last_frame_index: u64,
    prev_video_sequence: u32,
    nal_parser: NalParser<F>,
}

impl<F> StreamHandler<F> where F : Fn(Nal) {
    pub fn new(
        enable_fec: bool,
        codec: AlvrCodec,
        push_nal: F
    ) -> StreamHandler<F> {
        latency_controller::INSTANCE.lock().reset();
        let nal_parser = NalParser::new(
            enable_fec,
            codec,
            push_nal
        );
        StreamHandler {
            time_diff: 0,
            last_frame_index: 0,
            prev_video_sequence: 0,
            nal_parser,
        }
    }

    pub fn legacy_receive(&mut self, buffer: Bytes) {
        match AlvrPacketType::from(buffer.clone().get_u32_le()) {
            AlvrPacketType::TimeSync =>
                self.process_time_sync(buffer.into()),
            AlvrPacketType::VideoFrame => {
                let payload = buffer.clone().split_off(mem::size_of::<VideoFrameHeader>());
                self.process_video_frame(buffer.into(), payload)
            },
            AlvrPacketType::HapticsFeedback =>
                self.process_haptics_feedback(buffer.into()),
            _ => {}
        }
    }

    fn process_video_frame(
        &mut self, video_frame_header: VideoFrameHeader, video_frame_buffer: Bytes,
    ) {
        info!("{:?}", video_frame_header);

        let tracking_frame_index = video_frame_header.tracking_frame_index;
        if self.last_frame_index != tracking_frame_index {
            let mut latency_controller = latency_controller::INSTANCE.lock();

            latency_controller.received_first(tracking_frame_index);
            // FIXME Isn't it negative when the value of u64 is large?
            let t1 = video_frame_header.sent_time as i64 - self.time_diff;
            let t2 = util::get_timestamp_us() as i64;
            if t1 > t2 {
                latency_controller.estimated_sent(tracking_frame_index, 0);
            } else {
                latency_controller.estimated_sent(tracking_frame_index, (t2 - t1) as u64);
            }
            self.last_frame_index = tracking_frame_index
        }

        self.process_video_sequence(video_frame_header.packet_counter);

        {
            let mut latency_controller = latency_controller::INSTANCE.lock();

            let mut fec_failure = latency_controller.get_fec_failure_state();
            let processed = self.nal_parser.process_packet(
                video_frame_header, video_frame_buffer, &mut fec_failure);
            if processed {
                latency_controller.received_last(tracking_frame_index);
            }
            if fec_failure {
                latency_controller.fec_failure();
                Self::send_packet_error_report(AlvrLostFrameType::Video, 0, 0);
            }
            latency_controller.set_fec_failure_state(fec_failure);
        }
    }

    fn process_video_sequence(&mut self, sequence: u32) {
        // FIXME prev_video_sequence overflow
        let expected_video_sequence = self.prev_video_sequence + 1;
        if self.prev_video_sequence != 0 && expected_video_sequence != sequence {
            let lost = if expected_video_sequence < sequence {
                sequence - expected_video_sequence
            } else {
                // out-of-order
                // FIXME This is not accurate statistics.
                expected_video_sequence - sequence
            };

            let mut latency_controller = latency_controller::INSTANCE.lock();
            latency_controller.packet_loss(lost);

            error!("VideoPacket loss {} ({} -> {})", lost, expected_video_sequence, sequence - 1)
        }
        self.prev_video_sequence = sequence;
    }

    fn process_time_sync(&mut self, time_sync: TimeSync) {
        info!("{:?}", time_sync);

        let current = util::get_timestamp_us();
        match time_sync.mode {
            1 => {
                let mut latency_controller = latency_controller::INSTANCE.lock();
                latency_controller.set_total_latency(time_sync.server_total_latency);

                // FIXME Isn't it negative when the value of u64 is large?
                let rtt = current - time_sync.client_time;
                self.time_diff =
                    time_sync.server_time as i64 + rtt as i64 / 2 - current as i64;
                info!("TimeSync: server - client = {} us RTT = {} us", self.time_diff, rtt);
                Self::send_time_sync(time_sync, current);
            }
            3 => {
                let mut latency_controller = latency_controller::INSTANCE.lock();
                latency_controller.received(
                    time_sync.tracking_recv_frame_index, time_sync.server_time);
            }
            _ => {}
        }
    }

    fn process_haptics_feedback(&mut self, haptics_feedback: HapticsFeedback) {
        info!("{:?}", haptics_feedback);

        // self.activity.on_haptics_feedback(
        //     &self.vm,
        //     haptics_feedback.start_time,
        //     haptics_feedback.amplitude,
        //     haptics_feedback.duration,
        //     haptics_feedback.frequency,
        //     haptics_feedback.hand,
        // );
    }

    fn send_packet_error_report(
        frame_type: AlvrLostFrameType, from_packet_counter: u32, to_packet_counter: u32,
    ) {
        let packet_error_report = PacketErrorReport {
            lost_frame_type: frame_type.into(),
            from_packet_counter,
            to_packet_counter,
            ..Default::default()
        };
        // connection::legacy_send(packet_error_report.into());
    }

    fn send_time_sync(mut time_sync: TimeSync, client_time: u64) {
        time_sync.mode = 2;
        time_sync.client_time = client_time;
        // connection::legacy_send(time_sync.into());
    }
}

// impl Drop for StreamHandler {
//     fn drop(&mut self) {
//         self.activity.on_disconnected(&self.vm);
//     }
// }