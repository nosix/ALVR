use alvr_common::prelude::*;

pub struct FrameMap<const S: usize> {
    map: [u64; S],
}

impl<const S: usize> FrameMap<S> {
    pub fn new() -> FrameMap<S> {
        if !Self::is_pow2(S) {
            panic!("FrameMap size must be power of two.");
        }
        FrameMap {
            map: [0; S]
        }
    }

    #[inline(always)]
    fn is_pow2(n: usize) -> bool {
        if n == 0 {
            false
        } else {
            (n & (n - 1)) == 0
        }
    }

    pub fn insert(&mut self, presentation_time_us: i64, frame_index: u64) {
        let key = presentation_time_us as usize & (S - 1);
        if frame_index == 0 {
            warn!("0 means no value, ignore if frame_index is 0");
        }
        self.map[key] = frame_index;
    }

    pub fn remove(&mut self, presentation_time_us: i64) -> Option<u64> {
        let key = presentation_time_us as usize & (S - 1);
        let frame_index = self.map[key];
        self.map[key] = 0;

        if frame_index != 0 {
            Some(frame_index)
        } else {
            None
        }
    }
}