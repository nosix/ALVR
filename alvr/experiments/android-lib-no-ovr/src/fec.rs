use crate::legacy_packets::{VideoFrameHeader, ALVR_MAX_PACKET_SIZE};
use alvr_common::prelude::*;
use bytes::Bytes;
use reed_solomon_erasure::galois_8::ReedSolomon;
use std::{mem, ptr};

const ALVR_MAX_VIDEO_BUFFER_SIZE: usize = ALVR_MAX_PACKET_SIZE - mem::size_of::<VideoFrameHeader>();
const ALVR_FEC_SHARDS_MAX: usize = 20;

pub enum ReconstructError {
    NoOp,
    NotEnoughParity,
    ReconstructFailed,
}

#[derive(Debug)]
pub struct FecQueue {
    current_frame: VideoFrameHeader,
    shard_packets: usize,
    block_size: usize,
    total_data_shards: usize,
    total_parity_shards: usize,
    total_shards: usize,
    first_packet_of_next_frame: u32,
    marks: Vec<Vec<u8>>,
    frame_buffer: Vec<u8>,
    received_data_shards: Vec<u32>,
    received_parity_shard: Vec<u32>,
    recovered_packet: Vec<bool>,
    shards: Vec<Vec<u8>>,
    recovered: bool,
    rs: Option<ReedSolomon>,
}

impl FecQueue {
    pub fn new() -> FecQueue {
        FecQueue {
            current_frame: VideoFrameHeader {
                video_frame_index: u64::MAX,
                ..Default::default()
            },
            shard_packets: 0,
            block_size: 0,
            total_data_shards: 0,
            total_parity_shards: 0,
            total_shards: 0,
            first_packet_of_next_frame: 0,
            marks: Vec::with_capacity(0),
            frame_buffer: Vec::with_capacity(0),
            received_data_shards: Vec::with_capacity(0),
            received_parity_shard: Vec::with_capacity(0),
            recovered_packet: Vec::with_capacity(0),
            shards: Vec::with_capacity(0),
            recovered: true,
            rs: None,
        }
    }

    pub fn add_video_packet(&mut self, header: VideoFrameHeader, payload: &Bytes) -> StrResult {
        let mut fec_failure = false;

        if self.recovered &&
            self.current_frame.video_frame_index == header.video_frame_index
        {
            return Ok(());
        }

        let packet_counter = header.packet_counter as usize;
        let fec_index = header.fec_index as usize;

        if self.current_frame.video_frame_index != header.video_frame_index {
            if self.current_frame.tracking_frame_index == header.tracking_frame_index {
                // FIXME This causes problems with latency_controller
                warn!("tracking_frame_index has not been changed.");
            }
            // Check previous frame
            if !self.recovered {
                debug!("Previous frame cannot be recovered.");
                self.debug();
                fec_failure = true;
            }

            // New frame
            self.current_frame = header;
            self.recovered = false;

            self.shard_packets = FecQueue::calculate_fec_shard_packets(
                self.current_frame.frame_byte_size, self.current_frame.fec_percentage);
            self.block_size = self.shard_packets * ALVR_MAX_VIDEO_BUFFER_SIZE;
            self.total_data_shards =
                (self.current_frame.frame_byte_size as usize + self.block_size - 1)
                    / self.block_size;
            self.total_parity_shards = FecQueue::calculate_parity_shards(
                self.total_data_shards as u32, self.current_frame.fec_percentage) as usize;
            self.total_shards = self.total_data_shards + self.total_parity_shards;

            FecQueue::clear_vec(&mut self.recovered_packet, self.shard_packets, false);
            FecQueue::clear_vec(&mut self.received_data_shards, self.shard_packets, 0);
            FecQueue::clear_vec(&mut self.received_parity_shard, self.shard_packets, 0);
            FecQueue::clear_vec(&mut self.shards, self.total_shards, Vec::with_capacity(0));

            self.rs = Some(
                trace_err!(ReedSolomon::new(self.total_data_shards, self.total_parity_shards))?
            );

            FecQueue::clear_vec(&mut self.marks, self.shard_packets, Vec::with_capacity(0));
            for i in 0..self.shard_packets {
                FecQueue::clear_vec(&mut self.marks[i], self.total_shards, 1);
            }

            let required_buffer_size = self.total_shards * self.block_size;
            if self.frame_buffer.len() < required_buffer_size {
                // Only expand buffer for performance reason.
                debug!("Resize frame_buffer to {}", required_buffer_size);
                self.frame_buffer.resize(required_buffer_size, 0);
            }

            // Padding packets are not sent, so we can fill bitmap by default.
            let fec_data_packets =
                (self.current_frame.frame_byte_size as usize + ALVR_MAX_VIDEO_BUFFER_SIZE - 1)
                    / ALVR_MAX_VIDEO_BUFFER_SIZE;
            let padding =
                (self.shard_packets - fec_data_packets % self.shard_packets) % self.shard_packets;
            for i in 0..padding {
                self.marks[self.shard_packets - i - 1][self.total_shards - 1] = 0;
                self.received_data_shards[self.shard_packets - i - 1] += 1;
            }

            // Calculate last packet counter of current frame to detect whole frame packet loss.
            let (start_packet, next_start_packet) =
                if fec_index / self.shard_packets < self.total_data_shards
                {
                    // First seen packet was data packet
                    let start_packet = packet_counter - fec_index;
                    let next_start_packet = start_packet
                        + self.total_shards * self.shard_packets
                        - padding;
                    (start_packet, next_start_packet)
                } else {
                    // was parity packet
                    let start_packet = packet_counter
                        - (fec_index - padding);
                    let start_of_parity_packet = packet_counter
                        - (fec_index - self.total_data_shards * self.shard_packets);
                    let next_start_packet =
                        start_of_parity_packet + self.total_parity_shards * self.shard_packets;
                    (start_packet, next_start_packet)
                };

            if self.first_packet_of_next_frame != 0
                && self.first_packet_of_next_frame != start_packet as u32
            {
                // Whole frame packet loss
                debug!("Previous frame was completely lost. start_packet={}", start_packet);
                self.debug();
                fec_failure = true;
            }

            self.first_packet_of_next_frame = next_start_packet as u32;

            debug!("Start new frame.");
            self.debug();
        }

        let shard_index = fec_index / self.shard_packets;
        let packet_index = fec_index % self.shard_packets;
        if self.marks[packet_index][shard_index] == 0 {
            // Duplicate packet.
            debug!("Packet duplication. packet_counter={}, fec_index={}", packet_counter, fec_index);
            return if !fec_failure { Ok(()) } else { Err("FEC failed".into()) };
        }

        self.marks[packet_index][shard_index] = 0;
        if shard_index < self.total_data_shards {
            self.received_data_shards[packet_index] += 1;
        } else {
            self.received_parity_shard[packet_index] += 1;
        }

        let start_index = fec_index * ALVR_MAX_VIDEO_BUFFER_SIZE;
        FecQueue::mem_copy(&payload, &mut self.frame_buffer, start_index);

        if payload.len() != ALVR_MAX_VIDEO_BUFFER_SIZE {
            // Fill padding
            let (_, p) = self.frame_buffer.split_at_mut(start_index + payload.len());
            p.fill(0);
        }

        Ok(())
    }

    /// Calculate how many packet is needed for make signal shard.
    fn calculate_fec_shard_packets(length: u32, fec_percentage: u16) -> usize {
        // This reed solomon implementation accept only 255 shards.
        // Normally, we use ALVR_MAX_VIDEO_BUFFER_SIZE as block_size and single packet becomes single shard.
        // If we need more than maxDataShards packets, we need to combine multiple packet to make single shrad.
        // NOTE: Moonlight seems to use only 255 shards for video frame.
        let fec_percentage = fec_percentage as usize;
        let max_data_shards =
            ((ALVR_FEC_SHARDS_MAX - 2) * 100 + 99 + fec_percentage) / (100 + fec_percentage);
        let min_block_size = (length as usize + max_data_shards - 1) / max_data_shards;
        let shard_packets =
            (min_block_size + ALVR_MAX_VIDEO_BUFFER_SIZE - 1) / ALVR_MAX_VIDEO_BUFFER_SIZE;
        assert!(
            max_data_shards
                + FecQueue::calculate_parity_shards(max_data_shards as u32, fec_percentage as u16) as usize
                <= ALVR_FEC_SHARDS_MAX);
        shard_packets
    }

    fn calculate_parity_shards(data_shards: u32, fec_percentage: u16) -> u32 {
        (data_shards * fec_percentage as u32 + 99) / 100
    }

    fn clear_vec<T>(target: &mut Vec<T>, new_len: usize, fill_value: T) where T: Clone {
        target.clear();
        target.resize(new_len, fill_value);
    }

    fn mem_copy(src: &Bytes, dst: &mut Vec<u8>, dst_offset: usize) {
        assert!(dst.len() >= src.len() + dst_offset);
        unsafe {
            let src_ptr = src.as_ref().as_ptr();
            let dst_ptr = dst.as_mut_ptr().offset(dst_offset as isize);
            ptr::copy_nonoverlapping(src_ptr, dst_ptr, src.len());
        }
    }

    pub fn reconstruct(&mut self) -> Result<(), ReconstructError> {
        if self.recovered {
            return Err(ReconstructError::NoOp);
        }

        let mut ret = true;

        // On server side, we encoded all buffer in one call of reed_solomon_encode.
        // But client side, we should split shards for more resilient recovery.
        for pi in 0..self.shard_packets {
            if self.recovered_packet[pi] {
                continue;
            }
            if self.received_data_shards[pi] == self.total_data_shards as u32 {
                // We've received a full packet with no need for FEC.
                debug!("No need for FEC. packet_index={}", pi);
                self.recovered_packet[pi] = true;
                continue;
            }

            let shards = self.received_data_shards[pi] + self.received_parity_shard[pi];
            if shards < self.total_data_shards as u32 {
                // Not enough parity data
                ret = false;
                continue;
            }

            debug!(
                "Recovering. packet_index={} received_data_shards={}/{} received_parity_shards={}/{}",
                pi,
                self.received_data_shards[pi], self.total_data_shards,
                self.received_parity_shard[pi], self.total_parity_shards
            );

            for i in 0..self.total_shards {
                let (_, p) = self.frame_buffer.split_at(
                    (i * self.shard_packets + pi) * ALVR_MAX_VIDEO_BUFFER_SIZE);
                self.shards[i].clear();
                self.shards[i].extend_from_slice(p);
            }

            let result = if let Some(ref rs) = self.rs {
                let mut shards: Vec<_> = self.shards.iter().cloned().map(Some).collect();
                trace_err!(rs.reconstruct(&mut shards))
            } else {
                Err("ReedSolomon does not exist.".into())
            };

            self.recovered_packet[pi] = true;

            // We should always provide enough parity to recover the missing data successfully.
            // If this fails, something is probably wrong with our FEC state.
            if result.is_err() {
                error!("ReedSolomon::reconstruct failed.");
                return Err(ReconstructError::ReconstructFailed);
            }
        }

        if ret {
            self.recovered = true;
            debug!("Frame was successfully recovered by FEC.");
            Ok(())
        } else {
            Err(ReconstructError::NotEnoughParity)
        }
    }

    pub fn get_frame_buffer(&self) -> Bytes {
        let (p, _) = self.frame_buffer.split_at(self.current_frame.frame_byte_size as usize);
        Bytes::copy_from_slice(p)
    }

    fn debug(&self) {
        debug!(
            "video_frame_index={} shards={}:{} frame_byte_size={} fec_percentage={} total_shards={} shard_packets={} block_size={} first_packet_of_next_frame={} current_packet={}",
            self.current_frame.video_frame_index,
            self.total_data_shards,
            self.total_parity_shards,
            self.current_frame.frame_byte_size,
            self.current_frame.fec_percentage,
            self.total_shards,
            self.shard_packets,
            self.block_size,
            self.first_packet_of_next_frame,
            self.current_frame.packet_counter
        );
        for i in 0..self.shard_packets {
            debug!(
                "packet_index={}, shards={}:{}",
                i,
                self.received_data_shards[i],
                self.received_parity_shard[i]
            );
        }
    }
}