use std::time::Instant;

const METER_CHANNELS: usize = 15;
const PEAK_HOLD_SECS: f32 = 1.5;
const PEAK_DECAY_DB_PER_SEC: f32 = 26.0;
const MIN_DB: f32 = -144.0;

#[derive(Debug, Clone)]
pub struct ChannelMeter {
    pub level_db: f32,
    pub peak_db: f32,
    peak_timestamp: Option<Instant>,
}

impl Default for ChannelMeter {
    fn default() -> Self {
        Self {
            level_db: MIN_DB,
            peak_db: MIN_DB,
            peak_timestamp: None,
        }
    }
}

impl ChannelMeter {
    pub fn update_level(&mut self, db: f32) {
        self.level_db = db;

        if db > self.peak_db {
            self.peak_db = db;
            self.peak_timestamp = Some(Instant::now());
        }
    }

    pub fn tick(&mut self, dt_secs: f32) {
        if let Some(ts) = self.peak_timestamp {
            let elapsed = ts.elapsed().as_secs_f32();
            if elapsed > PEAK_HOLD_SECS {
                let decay_time = elapsed - PEAK_HOLD_SECS;
                let decayed = self.peak_db - (decay_time * PEAK_DECAY_DB_PER_SEC);
                self.peak_db = decayed.max(self.level_db).max(MIN_DB);
                if self.peak_db <= MIN_DB {
                    self.peak_timestamp = None;
                }
            }
        }
        let _ = dt_secs;
    }

    pub fn level_normalized(&self) -> f32 {
        db_to_normalized(self.level_db)
    }

    pub fn peak_normalized(&self) -> f32 {
        db_to_normalized(self.peak_db)
    }

    pub fn reset(&mut self) {
        self.level_db = MIN_DB;
        self.peak_db = MIN_DB;
        self.peak_timestamp = None;
    }
}

fn db_to_normalized(db: f32) -> f32 {
    if db <= MIN_DB {
        return 0.0;
    }
    ((db - MIN_DB) / (10.0 - MIN_DB)).clamp(0.0, 1.0)
}

#[derive(Debug, Clone)]
pub struct MeterState {
    pub channels: [ChannelMeter; METER_CHANNELS],
    pub gain_reduction: u16,
    last_update: Option<Instant>,
    pub active: bool,
}

impl Default for MeterState {
    fn default() -> Self {
        Self {
            channels: std::array::from_fn(|_| ChannelMeter::default()),
            gain_reduction: 0,
            last_update: None,
            active: false,
        }
    }
}

impl MeterState {
    pub fn apply_update(&mut self, levels: &[f32; 15], gain_reduction: u16) {
        let now = Instant::now();
        let dt = self
            .last_update
            .map(|prev| now.duration_since(prev).as_secs_f32())
            .unwrap_or(0.0);

        for (i, &db) in levels.iter().enumerate() {
            self.channels[i].update_level(db);
            self.channels[i].tick(dt);
        }

        self.gain_reduction = gain_reduction;
        self.last_update = Some(now);
        self.active = true;
    }

    pub fn tick(&mut self) {
        if !self.active {
            return;
        }
        let dt = self
            .last_update
            .map(|prev| prev.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        for ch in &mut self.channels {
            ch.tick(dt);
        }
    }

    pub fn gain_reduction_for_channel(&self, mixer_index: u8) -> u8 {
        if mixer_index <= 6 {
            ((self.gain_reduction >> (mixer_index * 2)) & 0x03) as u8
        } else {
            ((self.gain_reduction >> 14) & 0x03) as u8
        }
    }

    pub fn input_level(&self, input_idx: usize) -> &ChannelMeter {
        &self.channels[input_idx.min(METER_CHANNELS - 1)]
    }

    pub fn stereo_left(&self, input_idx: usize) -> &ChannelMeter {
        match input_idx {
            0..=3 => &self.channels[input_idx],
            4 => &self.channels[4],
            5 => &self.channels[6],
            6 => &self.channels[8],
            _ => &self.channels[0],
        }
    }

    pub fn stereo_right(&self, input_idx: usize) -> Option<&ChannelMeter> {
        match input_idx {
            4 => Some(&self.channels[5]),
            5 => Some(&self.channels[7]),
            6 => Some(&self.channels[9]),
            _ => None,
        }
    }

    pub fn reset(&mut self) {
        for ch in &mut self.channels {
            ch.reset();
        }
        self.gain_reduction = 0;
        self.active = false;
        self.last_update = None;
    }
}

pub const METER_REQUEST_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
pub const METER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_meter_is_min_db() {
        let meter = ChannelMeter::default();
        assert_eq!(meter.level_db, MIN_DB);
        assert_eq!(meter.peak_db, MIN_DB);
        assert_eq!(meter.level_normalized(), 0.0);
    }

    #[test]
    fn level_update_sets_peak() {
        let mut meter = ChannelMeter::default();
        meter.update_level(-10.0);
        assert_eq!(meter.level_db, -10.0);
        assert_eq!(meter.peak_db, -10.0);
    }

    #[test]
    fn peak_holds_above_level() {
        let mut meter = ChannelMeter::default();
        meter.update_level(0.0);
        meter.update_level(-20.0);
        assert_eq!(meter.peak_db, 0.0);
        assert_eq!(meter.level_db, -20.0);
    }

    #[test]
    fn normalized_range() {
        assert_eq!(db_to_normalized(MIN_DB), 0.0);
        assert_eq!(db_to_normalized(10.0), 1.0);
        let mid = db_to_normalized(0.0);
        assert!(mid > 0.9 && mid < 1.0);
    }

    #[test]
    fn gain_reduction_extraction() {
        let state = MeterState {
            gain_reduction: 0b0000_0000_0000_1011,
            ..Default::default()
        };
        assert_eq!(state.gain_reduction_for_channel(0), 3);
        assert_eq!(state.gain_reduction_for_channel(1), 2);
        assert_eq!(state.gain_reduction_for_channel(2), 0);
    }

    #[test]
    fn stereo_channel_mapping() {
        let mut state = MeterState::default();
        let mut levels = [MIN_DB; 15];
        levels[4] = -5.0;
        levels[5] = -10.0;
        state.apply_update(&levels, 0);

        assert_eq!(state.stereo_left(4).level_db, -5.0);
        assert_eq!(state.stereo_right(4).unwrap().level_db, -10.0);
        assert!(state.stereo_right(0).is_none());
    }

    #[test]
    fn meter_state_apply_update() {
        let mut state = MeterState::default();
        let levels = [0.0f32; 15];
        state.apply_update(&levels, 42);
        assert!(state.active);
        assert_eq!(state.gain_reduction, 42);
        for ch in &state.channels {
            assert_eq!(ch.level_db, 0.0);
        }
    }

    #[test]
    fn reset_clears_all() {
        let mut state = MeterState::default();
        let levels = [5.0f32; 15];
        state.apply_update(&levels, 100);
        state.reset();
        assert!(!state.active);
        assert_eq!(state.gain_reduction, 0);
        for ch in &state.channels {
            assert_eq!(ch.level_db, MIN_DB);
        }
    }
}
