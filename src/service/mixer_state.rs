use crate::model::channels::BusType;
use crate::model::flow8::FLOW8Controller;
use crate::service::ble_codec::{CodecError, PayloadReader};

const INPUT_COUNT: usize = 7;
pub const BLE_MAIN: u8 = 0x0f;
pub const BLE_MON1: u8 = 0x0a;
pub const BLE_MON2: u8 = 0x0b;
pub const BLE_FX1: u8 = 0x0c;
pub const BLE_FX2: u8 = 0x0d;

const LEVEL_DB_MIN: f32 = -70.0;
const LEVEL_DB_MAX: f32 = 10.0;
const GAIN_MIN: f32 = -20.0;
const GAIN_MAX: f32 = 60.0;
const EQ_GAIN_MIN: f32 = -15.0;
const EQ_GAIN_MAX: f32 = 15.0;
const LIMITER_MIN: f32 = -60.0;
const LIMITER_MAX: f32 = 0.0;
const OUTPUT_COUNT: usize = 3;
const FX_COUNT: usize = 2;
const PEQ_BANDS: usize = 4;
const GEQ_BANDS: usize = 9;
const INPUT_GAINS_PER_OUTPUT: usize = 7;
const INPUT_GAINS_PER_FX: usize = 7;
const AUX_GAINS_PER_FX: usize = 2;

// --- Decoded data structures ---

#[derive(Debug, Clone, Default)]
pub struct InputChannelData {
    pub id: u8,
    pub muted: bool,
    pub hpf_on: bool,
    pub phase: bool,
    pub phantom_power: bool,
    pub connected_l: bool,
    pub connected_r: bool,
    pub solo: bool,
    pub reserved: bool,
    pub gain_db: f32,
    pub squeezer: f32,
    pub hpf_frequency: u16,
    pub balance: f32,
    pub peq_gain: [f32; PEQ_BANDS],
    pub peq_freq: [u16; PEQ_BANDS],
    pub peq_q: [f32; PEQ_BANDS],
    pub label_id: u8,
    pub label_icon: u16,
    pub label_text: String,
}

#[derive(Debug, Clone, Default)]
pub struct OutputChannelData {
    pub id: u8,
    pub muted: bool,
    pub solo: bool,
    pub volume_db: f32,
    pub limiter_db: f32,
    pub pan: f32,
    pub input_gains: [f32; INPUT_GAINS_PER_OUTPUT],
    pub geq_gain: [f32; GEQ_BANDS],
    pub geq_freq: [u16; GEQ_BANDS],
    pub geq_q: [f32; GEQ_BANDS],
    pub delay: u32,
}

#[derive(Debug, Clone, Default)]
pub struct FxChannelData {
    pub id: u8,
    pub muted: bool,
    pub enable_main: bool,
    pub enable_mon1: bool,
    pub enable_mon2: bool,
    pub volume_db: f32,
    pub pan: f32,
    pub val1: u8,
    pub val2: u8,
    pub val3: u8,
    pub preset: u8,
    pub input_gains: [f32; INPUT_GAINS_PER_FX],
    pub aux_gains: [f32; AUX_GAINS_PER_FX],
}

#[derive(Debug, Clone, Default)]
pub struct MixerStateData {
    pub inputs: Vec<InputChannelData>,
    pub outputs: Vec<OutputChannelData>,
    pub fx_channels: Vec<FxChannelData>,
    pub headphone_volume_db: f32,
    pub btusb: bool,
    pub phones_only: bool,
    pub foot_pedal_switch: bool,
    pub mute_inputs: bool,
    pub usb_streaming_mode: bool,
    pub ch_56_usb_12: bool,
    pub ch_78_usb_34: bool,
    pub mon_stereo_link: bool,
    pub hp_use_mon: bool,
    pub hp_use_post: bool,
    pub main_10dbv_pad: bool,
    pub mon_10dbv_pad: bool,
}

// --- Decoder ---

pub fn decode_mixer_state(payload: &[u8]) -> Result<MixerStateData, CodecError> {
    let mut r = PayloadReader::new(payload);
    let mut state = MixerStateData::default();

    for _ in 0..INPUT_COUNT {
        state.inputs.push(decode_input_channel(&mut r)?);
    }

    for _ in 0..OUTPUT_COUNT {
        state.outputs.push(decode_output_channel(&mut r)?);
    }

    for _ in 0..FX_COUNT {
        state.fx_channels.push(decode_fx_channel(&mut r)?);
    }

    if r.remaining() > 0 {
        state.headphone_volume_db = r.read_uf8(3)?;
        state.btusb = r.read_bool()?;
        state.phones_only = r.read_bool()?;
        state.foot_pedal_switch = r.read_bool()?;
        state.mute_inputs = r.read_bool()?;
        state.usb_streaming_mode = r.read_bool()?;
        state.ch_56_usb_12 = r.read_bool()?;
        state.ch_78_usb_34 = r.read_bool()?;
        state.mon_stereo_link = r.read_bool()?;
        state.hp_use_mon = r.read_bool()?;
        state.hp_use_post = r.read_bool()?;
        state.main_10dbv_pad = r.read_bool()?;
        state.mon_10dbv_pad = r.read_bool()?;
    }

    Ok(state)
}

pub fn db_to_level_cc(db: f32) -> u8 {
    if db < LEVEL_DB_MIN {
        return 0;
    }
    let clamped = db.clamp(LEVEL_DB_MIN, LEVEL_DB_MAX);
    let cc = 1.0 + ((clamped - LEVEL_DB_MIN) / (LEVEL_DB_MAX - LEVEL_DB_MIN)) * 126.0;
    cc.round().clamp(0.0, 127.0) as u8
}

pub fn level_cc_to_db(cc: u8) -> f32 {
    if cc == 0 {
        return -144.0;
    }
    let t = (cc as f32 - 1.0) / 126.0;
    LEVEL_DB_MIN + t * (LEVEL_DB_MAX - LEVEL_DB_MIN)
}

pub fn db_to_gain_cc(db: f32) -> u8 {
    range_to_cc(db, GAIN_MIN, GAIN_MAX)
}

pub fn gain_cc_to_db(cc: u8) -> f32 {
    cc_to_range(cc, GAIN_MIN, GAIN_MAX)
}

pub fn balance_to_cc(value: f32) -> u8 {
    range_to_cc(value, -1.0, 1.0)
}

pub fn cc_to_balance(cc: u8) -> f32 {
    cc_to_range(cc, -1.0, 1.0)
}

pub fn db_to_eq_cc(db: f32) -> u8 {
    range_to_cc(db, EQ_GAIN_MIN, EQ_GAIN_MAX)
}

pub fn db_to_limiter_cc(db: f32) -> u8 {
    range_to_cc(db, LIMITER_MIN, LIMITER_MAX)
}

pub fn limiter_cc_to_db(cc: u8) -> f32 {
    cc_to_range(cc, LIMITER_MIN, LIMITER_MAX)
}

fn range_to_cc(value: f32, min: f32, max: f32) -> u8 {
    let clamped = value.clamp(min, max);
    let normalized = (clamped - min) / (max - min);
    (normalized * 127.0).round() as u8
}

fn cc_to_range(cc: u8, min: f32, max: f32) -> f32 {
    min + (cc as f32 / 127.0) * (max - min)
}

pub fn ui_bus_id_to_ble(bus_id: u8) -> u8 {
    match bus_id {
        7 => BLE_MAIN,
        8 => BLE_MON1,
        9 => BLE_MON2,
        10 => BLE_FX1,
        11 => BLE_FX2,
        other => other,
    }
}

pub fn apply_mixer_state(data: &MixerStateData, controller: &mut FLOW8Controller) {
    for input in &data.inputs {
        let idx = input.id as usize;
        if idx >= controller.channels.len() {
            continue;
        }
        let ch = &mut controller.channels[idx];
        ch.is_muted = input.muted;
        ch.is_soloed = input.solo;
        ch.phantom_pwr.is_on = input.phantom_power;
        ch.connected_l = input.connected_l;
        ch.connected_r = input.connected_r;
        ch.channel_strip.gain = db_to_gain_cc(input.gain_db);
        ch.channel_strip.balance = balance_to_cc(input.balance);
        ch.channel_strip.compressor = range_to_cc(input.squeezer, 0.0, 1.0);
        ch.four_band_eq.low = db_to_eq_cc(input.peq_gain[0]);
        ch.four_band_eq.low_mid = db_to_eq_cc(input.peq_gain[1]);
        ch.four_band_eq.hi_mid = db_to_eq_cc(input.peq_gain[2]);
        ch.four_band_eq.hi = db_to_eq_cc(input.peq_gain[3]);
        if !input.label_text.is_empty() {
            ch.name = input.label_text.clone();
        }
    }

    if let Some(main) = data.outputs.first() {
        for (i, &db) in main.input_gains.iter().enumerate() {
            if i < controller.channels.len() {
                controller.channels[i].channel_strip.level = db_to_level_cc(db);
            }
        }
    }

    if data.outputs.len() >= 2 {
        for (i, &db) in data.outputs[1].input_gains.iter().enumerate() {
            if i < controller.channels.len() {
                controller.channels[i].sends.mon1 = db_to_level_cc(db);
            }
        }
    }
    if data.outputs.len() >= 3 {
        for (i, &db) in data.outputs[2].input_gains.iter().enumerate() {
            if i < controller.channels.len() {
                controller.channels[i].sends.mon2 = db_to_level_cc(db);
            }
        }
    }

    if let Some(fx) = data.fx_channels.first() {
        for (i, &db) in fx.input_gains.iter().enumerate() {
            if i < controller.channels.len() {
                controller.channels[i].sends.fx1 = db_to_level_cc(db);
            }
        }
    }
    if data.fx_channels.len() >= 2 {
        for (i, &db) in data.fx_channels[1].input_gains.iter().enumerate() {
            if i < controller.channels.len() {
                controller.channels[i].sends.fx2 = db_to_level_cc(db);
            }
        }
    }

    for output in &data.outputs {
        let bus_type = match output.id {
            BLE_MAIN => BusType::Main,
            BLE_MON1 | BLE_MON2 => BusType::Monitor,
            _ => continue,
        };
        let bus = controller.buses.iter_mut().find(|b| match (bus_type, output.id) {
            (BusType::Main, _) => b.bus_type == BusType::Main,
            (BusType::Monitor, BLE_MON1) => b.bus_type == BusType::Monitor && b.index == 1,
            (BusType::Monitor, BLE_MON2) => b.bus_type == BusType::Monitor && b.index == 2,
            _ => false,
        });
        if let Some(bus) = bus {
            bus.bus_strip.level = db_to_level_cc(output.volume_db);
            bus.bus_strip.balance = balance_to_cc(output.pan);
            bus.bus_strip.limiter = db_to_limiter_cc(output.limiter_db);
            for (i, &gain) in output.geq_gain.iter().enumerate() {
                let cc = db_to_eq_cc(gain);
                match i {
                    0 => bus.nine_band_eq.freq_62_hz = cc,
                    1 => bus.nine_band_eq.freq_125_hz = cc,
                    2 => bus.nine_band_eq.freq_250_hz = cc,
                    3 => bus.nine_band_eq.freq_500_hz = cc,
                    4 => bus.nine_band_eq.freq_1_khz = cc,
                    5 => bus.nine_band_eq.freq_2_khz = cc,
                    6 => bus.nine_band_eq.freq_4_khz = cc,
                    7 => bus.nine_band_eq.freq_8_khz = cc,
                    8 => bus.nine_band_eq.freq_16_khz = cc,
                    _ => {}
                }
            }
        }
    }

    for (i, fx) in data.fx_channels.iter().enumerate() {
        if i >= controller.fx_slots.len() {
            continue;
        }
        let slot = &mut controller.fx_slots[i];
        slot.preset = fx.preset.min(15);
        slot.param1 = fx.val1;
        slot.param2 = fx.val2;
        if let Some(bus) = controller.buses.iter_mut().find(|b| {
            b.bus_type == BusType::Fx && b.index == (i as u8 + 3)
        }) {
            bus.bus_strip.level = db_to_level_cc(fx.volume_db);
            bus.bus_strip.balance = balance_to_cc(fx.pan);
        }
    }

    controller.mark_all_synced();
}

#[allow(clippy::field_reassign_with_default)]
fn decode_input_channel(r: &mut PayloadReader) -> Result<InputChannelData, CodecError> {
    let mut ch = InputChannelData::default();

    ch.id = r.read_u8()?;

    ch.muted = r.read_bool()?;
    ch.hpf_on = r.read_bool()?;
    ch.phase = r.read_bool()?;
    ch.phantom_power = r.read_bool()?;
    ch.connected_l = r.read_bool()?;
    ch.connected_r = r.read_bool()?;
    ch.solo = r.read_bool()?;
    ch.reserved = r.read_bool()?;

    ch.gain_db = r.read_uf8(5)?;
    ch.squeezer = r.read_uf8(2)?;
    ch.hpf_frequency = r.read_u16be()?;
    ch.balance = r.read_uf8(1)?;

    for i in 0..PEQ_BANDS {
        ch.peq_gain[i] = r.read_uf8(4)?;
    }
    for i in 0..PEQ_BANDS {
        ch.peq_freq[i] = r.read_u16be()?;
    }
    for i in 0..PEQ_BANDS {
        ch.peq_q[i] = r.read_uf8(6)?;
    }

    ch.label_id = r.read_u8()?;
    ch.label_icon = r.read_u16be()?;
    ch.label_text = r.read_string()?;

    Ok(ch)
}

#[allow(clippy::field_reassign_with_default)]
fn decode_output_channel(r: &mut PayloadReader) -> Result<OutputChannelData, CodecError> {
    let mut ch = OutputChannelData::default();

    ch.id = r.read_u8()?;

    ch.muted = r.read_bool()?;
    ch.solo = r.read_bool()?;

    ch.volume_db = r.read_uf8(3)?;
    ch.limiter_db = r.read_uf8(5)?;
    ch.pan = r.read_uf8(1)?;

    for i in 0..INPUT_GAINS_PER_OUTPUT {
        ch.input_gains[i] = r.read_uf8(3)?;
    }

    for i in 0..GEQ_BANDS {
        ch.geq_gain[i] = r.read_uf8(4)?;
    }
    for i in 0..GEQ_BANDS {
        ch.geq_freq[i] = r.read_u16be()?;
    }
    for i in 0..GEQ_BANDS {
        ch.geq_q[i] = r.read_uf8(6)?;
    }

    ch.delay = r.read_u32be()?;

    Ok(ch)
}

#[allow(clippy::field_reassign_with_default)]
fn decode_fx_channel(r: &mut PayloadReader) -> Result<FxChannelData, CodecError> {
    let mut ch = FxChannelData::default();

    ch.id = r.read_u8()?;

    ch.muted = r.read_bool()?;
    ch.enable_main = r.read_bool()?;
    ch.enable_mon1 = r.read_bool()?;
    ch.enable_mon2 = r.read_bool()?;

    ch.volume_db = r.read_uf8(3)?;
    ch.pan = r.read_uf8(1)?;

    ch.val1 = r.read_u8()?;
    ch.val2 = r.read_u8()?;
    ch.val3 = r.read_u8()?;
    ch.preset = r.read_u8()?;

    for i in 0..INPUT_GAINS_PER_FX {
        ch.input_gains[i] = r.read_uf8(3)?;
    }
    for i in 0..AUX_GAINS_PER_FX {
        ch.aux_gains[i] = r.read_uf8(3)?;
    }

    Ok(ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ble_codec::{uf8_encode, PayloadWriter};

    fn build_input_channel(id: u8, muted: bool, gain_db: f32, balance: f32) -> Vec<u8> {
        let mut w = PayloadWriter::new();
        w.write_u8(id);

        w.write_bool(muted);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(true);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);

        let gain_raw = uf8_encode(5, gain_db).unwrap();
        w.write_raw(&[gain_raw]);
        let sq_raw = uf8_encode(2, 0.5).unwrap();
        w.write_raw(&[sq_raw]);
        w.write_u16be(80);
        let bal_raw = uf8_encode(1, balance).unwrap();
        w.write_raw(&[bal_raw]);

        for _ in 0..4 {
            let g = uf8_encode(4, 0.0).unwrap();
            w.write_raw(&[g]);
        }
        for _ in 0..4 {
            w.write_u16be(1000);
        }
        for _ in 0..4 {
            let q = uf8_encode(6, 0.5).unwrap();
            w.write_raw(&[q]);
        }

        w.write_u8(id);
        w.write_u16be(0);
        w.write_string("Ch");

        w.finish()
    }

    fn build_output_channel(id: u8, volume_db: f32) -> Vec<u8> {
        let mut w = PayloadWriter::new();
        w.write_u8(id);

        w.write_bool(false);
        w.write_bool(false);

        let vol = uf8_encode(3, volume_db).unwrap();
        w.write_raw(&[vol]);
        let lim = uf8_encode(5, 0.0).unwrap();
        w.write_raw(&[lim]);
        let pan = uf8_encode(1, 0.0).unwrap();
        w.write_raw(&[pan]);

        for _ in 0..7 {
            let g = uf8_encode(3, 0.0).unwrap();
            w.write_raw(&[g]);
        }
        for _ in 0..9 {
            let g = uf8_encode(4, 0.0).unwrap();
            w.write_raw(&[g]);
        }
        for _ in 0..9 {
            w.write_u16be(1000);
        }
        for _ in 0..9 {
            let q = uf8_encode(6, 0.5).unwrap();
            w.write_raw(&[q]);
        }

        w.write_u32be(0);

        w.finish()
    }

    fn build_fx_channel(id: u8, preset: u8) -> Vec<u8> {
        let mut w = PayloadWriter::new();
        w.write_u8(id);

        w.write_bool(false);
        w.write_bool(true);
        w.write_bool(false);
        w.write_bool(false);

        let vol = uf8_encode(3, 0.0).unwrap();
        w.write_raw(&[vol]);
        let pan = uf8_encode(1, 0.0).unwrap();
        w.write_raw(&[pan]);

        w.write_u8(1);
        w.write_u8(2);
        w.write_u8(3);
        w.write_u8(preset);

        for _ in 0..7 {
            let g = uf8_encode(3, 0.0).unwrap();
            w.write_raw(&[g]);
        }
        for _ in 0..2 {
            let g = uf8_encode(3, 0.0).unwrap();
            w.write_raw(&[g]);
        }

        w.finish()
    }

    fn build_full_mixer_state() -> Vec<u8> {
        let mut payload = Vec::new();

        for i in 0..7u8 {
            payload.extend(build_input_channel(i, i == 0, 0.0, 0.0));
        }
        for &id in &[0x0fu8, 0x0a, 0x0b] {
            payload.extend(build_output_channel(id, 0.0));
        }
        for (idx, &id) in [0x0cu8, 0x0d].iter().enumerate() {
            payload.extend(build_fx_channel(id, idx as u8));
        }

        let hp = uf8_encode(3, -10.0).unwrap();
        let mut w = PayloadWriter::new();
        w.write_raw(&[hp]);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        payload.extend(w.finish());

        payload
    }

    #[test]
    fn decode_full_mixer_state() {
        let payload = build_full_mixer_state();
        let state = decode_mixer_state(&payload).unwrap();

        assert_eq!(state.inputs.len(), 7);
        assert_eq!(state.outputs.len(), 3);
        assert_eq!(state.fx_channels.len(), 2);

        assert_eq!(state.inputs[0].id, 0);
        assert!(state.inputs[0].muted);
        assert!(!state.inputs[1].muted);

        assert!(state.inputs[0].connected_l);
        assert!(!state.inputs[0].connected_r);

        assert_eq!(state.outputs[0].id, 0x0f);
        assert_eq!(state.outputs[1].id, 0x0a);

        assert_eq!(state.fx_channels[0].id, 0x0c);
        assert_eq!(state.fx_channels[0].preset, 0);
        assert!(state.fx_channels[0].enable_main);
        assert!(!state.fx_channels[0].muted);
        assert_eq!(state.fx_channels[1].preset, 1);
    }

    #[test]
    fn decode_input_channel_labels() {
        let payload = build_full_mixer_state();
        let state = decode_mixer_state(&payload).unwrap();

        for (i, ch) in state.inputs.iter().enumerate() {
            assert_eq!(ch.label_text, "Ch");
            assert_eq!(ch.label_id, i as u8);
        }
    }

    #[test]
    fn decode_output_channel_geq() {
        let payload = build_full_mixer_state();
        let state = decode_mixer_state(&payload).unwrap();

        for out in &state.outputs {
            assert_eq!(out.geq_gain.len(), 9);
            assert_eq!(out.geq_freq.len(), 9);
            assert_eq!(out.geq_q.len(), 9);
            for &gain in &out.geq_gain {
                assert!(gain.abs() < 1.0);
            }
        }
    }

    #[test]
    fn decode_fx_channel_gains() {
        let payload = build_full_mixer_state();
        let state = decode_mixer_state(&payload).unwrap();

        for fx in &state.fx_channels {
            assert_eq!(fx.input_gains.len(), 7);
            assert_eq!(fx.aux_gains.len(), 2);
            assert_eq!(fx.val1, 1);
            assert_eq!(fx.val2, 2);
            assert_eq!(fx.val3, 3);
        }
    }

    #[test]
    fn decode_trailing_flags() {
        let payload = build_full_mixer_state();
        let state = decode_mixer_state(&payload).unwrap();

        let hp_db = state.headphone_volume_db;
        assert!((hp_db - (-10.0)).abs() < 1.0);
        assert!(!state.btusb);
        assert!(!state.phones_only);
    }

    #[test]
    fn truncated_payload_returns_error() {
        let result = decode_mixer_state(&[0x00]);
        assert!(result.is_err());
    }
}
