use std::time::{SystemTime, UNIX_EPOCH};

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0x9e3779b97f4a7c15 } else { seed };
        Self { state }
    }

    pub fn from_time() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let seed = duration.as_nanos() as u64 ^ 0x517cc1b727220a95;
        Self::new(seed)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut z = self.state.wrapping_add(0x9e3779b97f4a7c15);
        self.state = z;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    #[inline]
    pub fn gen_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16777216.0
    }

    #[inline]
    pub fn gen_range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.gen_f32()
    }
    #[inline]
    pub fn gen_range_usize(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        let range = (max - min) as u64;
        min + (self.next_u64() % range) as usize
    }
    #[inline]
    pub fn choice<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        assert!(!slice.is_empty(), "Cannot choose from empty slice");
        let idx = self.gen_range_usize(0, slice.len());
        &slice[idx]
    }
}
