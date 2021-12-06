use crate::{
    common::AlvrCodec,
    fec::FecQueue,
    legacy_packets::VideoFrameHeader,
};
use alvr_common::prelude::*;
use bytes::Bytes;

#[derive(Debug, PartialEq)]
pub enum NalType {
    Sps,
    Idr,
    P
}

const NAL_TYPE_SPS: u8 = 7;
const NAL_TYPE_IDR: u8 = 5;
const _NAL_TYPE_P: u8 = 1;

const H265_NAL_TYPE_IDR_W_RADL: u8 = 19;
const H265_NAL_TYPE_VPS:u8 = 32;

pub struct Nal {
    pub nal_type: NalType,
    pub frame_buffer: Bytes,
    pub frame_index: u64,
}

pub struct NalParser<F> where F : Fn(Nal) {
    enable_fec: bool,
    codec: AlvrCodec,
    push_nal: F,
    queue: FecQueue,
}

impl<F> NalParser<F> where F : Fn(Nal) {
    pub fn new(
        enable_fec: bool,
        codec: AlvrCodec,
        push_nal: F
    ) -> NalParser<F> {
        NalParser {
            enable_fec,
            codec,
            push_nal,
            queue: FecQueue::new(),
        }
    }

    pub fn process_packet(
        &mut self, frame_header: VideoFrameHeader, frame_buffer: Bytes, fec_failure: &mut bool,
    ) -> bool {
        let tracking_frame_index = frame_header.tracking_frame_index;

        if self.enable_fec {
            if let Err(e) = self.queue.add_video_packet(frame_header, &frame_buffer) {
                error!("add_video_packet return error '{}'", e);
                *fec_failure = true;
            }
        }

        if !self.queue.reconstruct() {
            return false;
        }

        let mut frame_buffer = if self.enable_fec {
            self.queue.get_frame_buffer()
        } else {
            frame_buffer
        };

        let nal_type = self.detect_nal_type(&frame_buffer);

        if nal_type == NalType::Sps {
            // This frame contains (VPS + )SPS + PPS + IDR on NVENC H.264 (H.265) stream.
            // (VPS + )SPS + PPS has short size (8bytes + 28bytes in some environment),
            // so we can assume SPS + PPS is contained in first fragment.
            let end = match self.find_vps_sps(frame_buffer.clone()) {
                Ok(end) => end,
                Err(_) => {
                    // Invalid frame.
                    error!("Got invalid frame. Too large SPS or PPS?");
                    return false;
                }
            };
            debug!("nal_type={:?} end={} codec={:?}", nal_type, end, self.codec);

            self.push(frame_buffer.split_to(end), tracking_frame_index);
            self.push(frame_buffer, tracking_frame_index);

            *fec_failure = false;
        } else {
            self.push(frame_buffer, tracking_frame_index);
        }

        true
    }

    fn find_vps_sps(&self, frame_buffer: Bytes) -> Result<usize, ()> {
        let mut zeros = 0;
        let mut found_nals = 0;
        for (i, b) in frame_buffer.iter().enumerate() {
            match b {
                0 => {
                    zeros += 1;
                }
                1 => {
                    if zeros >= 2 {
                        found_nals += 1;
                        match self.codec {
                            AlvrCodec::H264 if found_nals >= 3 => {
                                // Find end of SPS+PPS on H.264.
                                return Ok(i - 3);
                            }
                            AlvrCodec::H265 if found_nals >= 4 => {
                                // Find end of VPS+SPS+PPS on H.264.
                                return Ok(i - 3);
                            }
                            _ => {}
                        }
                    }
                    zeros = 0;
                }
                _ => {
                    zeros = 0;
                }
            }
        }
        Err(())
    }

    fn push(&self, frame_buffer: Bytes, frame_index: u64) {
        let nal_type = self.detect_nal_type(&frame_buffer);

        if frame_buffer.len() > 8 {
            debug!("push_nal {:?} len={} index={} buf=[{} {} {} {} .. {} {} {} {}]",
                  nal_type,
                  frame_buffer.len(),
                  frame_index,
                  frame_buffer[0], frame_buffer[1], frame_buffer[2], frame_buffer[3],
                  frame_buffer[frame_buffer.len() - 4],
                  frame_buffer[frame_buffer.len() - 3],
                  frame_buffer[frame_buffer.len() - 2],
                  frame_buffer[frame_buffer.len() - 1],
            );
        } else {
            debug!("push_nal {:?} len={} index={} buf={:?}",
                  nal_type,
                  frame_buffer.len(),
                  frame_index,
                  frame_buffer
            );
        }

        (self.push_nal)(Nal { nal_type, frame_buffer, frame_index });
    }

    fn detect_nal_type(&self, frame_buffer: &Bytes) -> NalType {
        let nal_type = match self.codec {
            AlvrCodec::H264 => frame_buffer[4] & 0x1F,
            AlvrCodec::H265 => (frame_buffer[4] >> 1) & 0x3F,
            AlvrCodec::Unknown => panic!("Unknown codec")
        };

        if (self.codec == AlvrCodec::H264 && nal_type == NAL_TYPE_SPS) ||
            (self.codec == AlvrCodec::H265 && nal_type == H265_NAL_TYPE_VPS) {
            // (VPS + )SPS + PPS
            NalType::Sps
        } else if (self.codec == AlvrCodec::H264 && nal_type == NAL_TYPE_IDR) ||
            (self.codec == AlvrCodec::H265 && nal_type == H265_NAL_TYPE_IDR_W_RADL) {
            // IDR-Frame
            NalType::Idr
        } else {
            // PFrame
            NalType::P
        }
    }
}