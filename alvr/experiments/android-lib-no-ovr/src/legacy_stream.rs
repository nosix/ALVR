use crate::{
    buffer_queue,
    common::AlvrCodec,
    fec::ReconstructError,
    latency_controller,
    legacy_packets::*,
    nal::{Nal, NalParser, ProcessError},
    util,
};
use alvr_common::prelude::*;
use bytes::{Bytes, Buf};
use std::mem;

pub struct StreamHandler<P, S> where P: Fn(Nal), S: Fn(Vec<u8>) {
    server_time_diff: i64,
    last_frame_index: u64,
    prev_video_sequence: u32,
    nal_parser: NalParser<P>,
    legacy_send: S,
}

impl<P, S> StreamHandler<P, S> where P: Fn(Nal), S: Fn(Vec<u8>) {
    pub fn new(
        enable_fec: bool,
        codec: AlvrCodec,
        push_nal: P,
        legacy_send: S,
    ) -> StreamHandler<P, S> {
        latency_controller::reset();
        let nal_parser = NalParser::new(
            enable_fec,
            codec,
            push_nal,
        );
        StreamHandler {
            server_time_diff: 0,
            last_frame_index: 0,
            prev_video_sequence: 0,
            nal_parser,
            legacy_send,
        }
    }

    pub fn legacy_receive(&mut self, buffer: Bytes) {
        match AlvrPacketType::from(buffer.clone().get_u32_le()) {
            AlvrPacketType::TimeSync =>
                self.process_time_sync(buffer.into()),
            AlvrPacketType::VideoFrame => {
                let payload = buffer.clone().split_off(mem::size_of::<VideoFrameHeader>());
                self.process_video_frame(buffer.into(), payload)
            }
            AlvrPacketType::HapticsFeedback =>
                self.process_haptics_feedback(buffer.into()),
            _ => {}
        }
    }

    fn process_video_frame(
        &mut self, video_frame_header: VideoFrameHeader, video_frame_buffer: Bytes,
    ) {
        debug!("{:?}", video_frame_header);

        let tracking_frame_index = video_frame_header.tracking_frame_index;
        if self.last_frame_index != tracking_frame_index {
            latency_controller::estimated_sent(
                tracking_frame_index,
                self.to_estimated_client_time(video_frame_header.sent_time),
            );
            latency_controller::received_first(tracking_frame_index);
            self.last_frame_index = tracking_frame_index
        }

        self.process_video_sequence(video_frame_header.packet_counter);

        {
            let mut fec_failure = latency_controller::get_fec_failure_state();
            let res = self.nal_parser.process_packet(
                video_frame_header, video_frame_buffer, &mut fec_failure,
            );
            match res {
                Ok(_) => {
                    latency_controller::received_last(tracking_frame_index);
                }
                Err(ProcessError::ReconstructFailed(ReconstructError::ReconstructFailed)) => {
                    buffer_queue::reset_idr_parsed();
                }
                _ => ()
            }
            if fec_failure {
                latency_controller::fec_failure();
                self.send_packet_error_report(AlvrLostFrameType::Video, 0, 0);
            }
            latency_controller::set_fec_failure_state(fec_failure);
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
                error!("VideoPacket out of order");
                expected_video_sequence - sequence
            };

            latency_controller::packet_loss(lost);

            error!("VideoPacket loss {} ({} -> {})", lost, self.prev_video_sequence, sequence)
        }
        self.prev_video_sequence = sequence;
    }

    fn process_time_sync(&mut self, time_sync: TimeSync) {
        debug!("{:?}", time_sync);

        let current = util::get_timestamp_us();
        match time_sync.mode {
            1 => {
                latency_controller::set_total_latency(time_sync.server_total_latency);

                let rtt = current - time_sync.client_time;
                self.set_server_time_diff(time_sync.server_time, current, rtt);
                self.send_time_sync(time_sync, current);
            }
            3 => {
                latency_controller::received(time_sync.tracking_recv_frame_index);
            }
            _ => {}
        }
    }

    fn process_haptics_feedback(&mut self, haptics_feedback: HapticsFeedback) {
        debug!("{:?}", haptics_feedback);

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
        &self,
        frame_type: AlvrLostFrameType,
        from_packet_counter: u32,
        to_packet_counter: u32,
    ) {
        let packet_error_report = PacketErrorReport {
            lost_frame_type: frame_type.into(),
            from_packet_counter,
            to_packet_counter,
            ..Default::default()
        };
        (self.legacy_send)(packet_error_report.into());
    }

    fn send_time_sync(
        &self,
        mut time_sync: TimeSync,
        client_time: u64,
    ) {
        time_sync.mode = 2;
        time_sync.client_time = client_time;
        debug!("TimeSync {:?}", time_sync);
        (self.legacy_send)(time_sync.into());
    }

    fn set_server_time_diff(&mut self, sent_server_time: u64, received_client_time: u64, rtt: u64) {
        self.server_time_diff =
            sent_server_time as i64 + rtt as i64 / 2 - received_client_time as i64;
        debug!("TimeSync: server - client = {} us RTT = {} us", self.server_time_diff, rtt);
    }

    fn to_estimated_client_time(&self, server_time: u64) -> u64 {
        (server_time as i64 - self.server_time_diff) as u64
    }
}

// impl Drop for StreamHandler {
//     fn drop(&mut self) {
//         self.activity.on_disconnected(&self.vm);
//     }
// }