use crate::model::flow8::FLOW8Controller;
use crate::{log, log_debug, log_warn};
#[cfg(any(debug_assertions, feature = "dev-tools"))]
use std::fs;
#[cfg(any(debug_assertions, feature = "dev-tools"))]
use std::path::PathBuf;

const SYSEX_START: u8 = 0xF0;
const SYSEX_END: u8 = 0xF7;
const BEHRINGER_ID: [u8; 3] = [0x00, 0x20, 0x32];
const FLOW8_MODEL: u8 = 0x21;
const MIN_DUMP_SIZE: usize = 100;

const LEVEL_DB_MIN: f32 = -70.0;
const LEVEL_DB_MAX: f32 = 10.0;
const GAIN_MIN: f32 = -20.0;
const GAIN_MAX: f32 = 60.0;
const EQ_MIN: f32 = -15.0;
const EQ_MAX: f32 = 15.0;
const PAN_MIN: f32 = -1.0;
const PAN_MAX: f32 = 1.0;
const COMP_MIN: f32 = 0.0;
const COMP_MAX: f32 = 1.0;
const BAL_MIN: f32 = -1.0;
const BAL_MAX: f32 = 1.0;
const LIMITER_MIN: f32 = -30.0;
const LIMITER_MAX: f32 = 0.0;
const LOWCUT_MIN_HZ: u16 = 20;
const LOWCUT_MAX_HZ: u16 = 600;

const NAMES_START: usize = 0x0554;
const NAMES_STRIDE: usize = 0x1E;
const NAME_SCAN_LEN: usize = 14;

struct FloatParam {
    msb_off: usize,
    data_offs: [usize; 4],
    bit_indices: [u8; 4],
}

// ── Channel parameters (7 channels: Ch1..Ch7/USB-BT) ──────────────────

const CHANNEL_LEVELS: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0067, data_offs: [0x0068, 0x0069, 0x006A, 0x006B], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x00B4, data_offs: [0x00B2, 0x00B3, 0x00B5, 0x00B6], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x00FA, data_offs: [0x00FD, 0x00FE, 0x00FF, 0x0100], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x0147, data_offs: [0x0148, 0x0149, 0x014A, 0x014B], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0194, data_offs: [0x0192, 0x0193, 0x0195, 0x0196], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x01DA, data_offs: [0x01DD, 0x01DE, 0x01DF, 0x01E0], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x0227, data_offs: [0x0228, 0x0229, 0x022A, 0x022B], bit_indices: [0, 1, 2, 3] },
];

const CHANNEL_GAINS: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0739, data_offs: [0x0736, 0x073A, 0x073B, 0x073C], bit_indices: [4, 0, 1, 2] },
    FloatParam { msb_off: 0x077A, data_offs: [0x077B, 0x077C, 0x077D, 0x077E], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x07BD, data_offs: [0x07BB, 0x07BC, 0x07BE, 0x07BF], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x07FD, data_offs: [0x07FA, 0x07FE, 0x07FF, 0x0800], bit_indices: [4, 0, 1, 2] },
    FloatParam { msb_off: 0x083E, data_offs: [0x083F, 0x0840, 0x0841, 0x0842], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0881, data_offs: [0x087F, 0x0880, 0x0882, 0x0883], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x08C1, data_offs: [0x08BE, 0x08C2, 0x08C3, 0x08C4], bit_indices: [4, 0, 1, 2] },
];

const CHANNEL_PANS: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0506, data_offs: [0x0501, 0x0502, 0x0503, 0x0508], bit_indices: [6, 5, 4, 1] },
    FloatParam { msb_off: 0x0506, data_offs: [0x0504, 0x0505, 0x0507, 0x050C], bit_indices: [3, 2, 0, 5] },
    FloatParam { msb_off: 0x050C, data_offs: [0x050A, 0x050B, 0x050D, 0x0511], bit_indices: [3, 2, 0, 4] },
    FloatParam { msb_off: 0x0514, data_offs: [0x050F, 0x0510, 0x0511, 0x0516], bit_indices: [6, 5, 4, 1] },
    FloatParam { msb_off: 0x0514, data_offs: [0x0512, 0x0513, 0x0515, 0x051A], bit_indices: [3, 2, 0, 5] },
    FloatParam { msb_off: 0x051A, data_offs: [0x0518, 0x0519, 0x051B, 0x051F], bit_indices: [3, 2, 0, 4] },
    FloatParam { msb_off: 0x0522, data_offs: [0x051D, 0x051E, 0x051F, 0x0524], bit_indices: [6, 5, 4, 1] },
];

const CHANNEL_COMPRESSORS: [FloatParam; 7] = [
    FloatParam { msb_off: 0x073D, data_offs: [0x073E, 0x073F, 0x0740, 0x0741], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x077C, data_offs: [0x077F, 0x0780, 0x0781, 0x0782], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x07C0, data_offs: [0x07C1, 0x07C2, 0x07C3, 0x07C4], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0801, data_offs: [0x0802, 0x0803, 0x0804, 0x0805], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0840, data_offs: [0x0843, 0x0844, 0x0845, 0x0846], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x0884, data_offs: [0x0885, 0x0886, 0x0887, 0x0888], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0884, data_offs: [0x0885, 0x0886, 0x0887, 0x0888], bit_indices: [0, 1, 2, 3] }, // usb-bt placeholder (skipped by has_compressor)
];

struct LowCutParam {
    lo_off: usize,
    hi_off: usize,
}

const CHANNEL_LOW_CUTS: [LowCutParam; 7] = [
    LowCutParam { lo_off: 0x0742, hi_off: 0x0743 },
    LowCutParam { lo_off: 0x0784, hi_off: 0x0785 },
    LowCutParam { lo_off: 0x07C5, hi_off: 0x07C6 },
    LowCutParam { lo_off: 0x0806, hi_off: 0x0807 },
    LowCutParam { lo_off: 0x0848, hi_off: 0x0849 },
    LowCutParam { lo_off: 0x0889, hi_off: 0x088A },
    LowCutParam { lo_off: 0, hi_off: 0 }, // USB-BT has no lowcut (skipped by has_low_cut)
];

const CHANNEL_EQ_LOW: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0744, data_offs: [0x0742, 0x0743, 0x0749, 0x074A], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x078A, data_offs: [0x0785, 0x0786, 0x078B, 0x078C], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x07C9, data_offs: [0x07C5, 0x07C6, 0x07CC, 0x07CD], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0808, data_offs: [0x0806, 0x0807, 0x080D, 0x080E], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x084E, data_offs: [0x0849, 0x084A, 0x084F, 0x0850], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x088D, data_offs: [0x0889, 0x088A, 0x0890, 0x0891], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x08CC, data_offs: [0x08CA, 0x08CB, 0x08D1, 0x08D2], bit_indices: [3, 2, 4, 5] },
];

const CHANNEL_EQ_LOW_MID: [FloatParam; 7] = [
    FloatParam { msb_off: 0x074B, data_offs: [0x0747, 0x0748, 0x074E, 0x074F], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x078A, data_offs: [0x0788, 0x0789, 0x078F, 0x0790], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x07D0, data_offs: [0x07CB, 0x07CC, 0x07D1, 0x07D2], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x080F, data_offs: [0x080B, 0x080C, 0x0812, 0x0813], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x084E, data_offs: [0x084C, 0x084D, 0x0853, 0x0854], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0894, data_offs: [0x088F, 0x0890, 0x0895, 0x0896], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x08D3, data_offs: [0x08CF, 0x08D0, 0x08D6, 0x08D7], bit_indices: [5, 4, 2, 3] },
];

const CHANNEL_EQ_HI_MID: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0752, data_offs: [0x074D, 0x074E, 0x0753, 0x0754], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0791, data_offs: [0x078D, 0x078E, 0x0794, 0x0795], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x07D0, data_offs: [0x07CE, 0x07CF, 0x07D5, 0x07D6], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0816, data_offs: [0x0811, 0x0812, 0x0817, 0x0818], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0855, data_offs: [0x0851, 0x0852, 0x0858, 0x0859], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0894, data_offs: [0x0892, 0x0893, 0x0899, 0x089A], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x08DA, data_offs: [0x08D5, 0x08D6, 0x08DB, 0x08DC], bit_indices: [6, 5, 0, 1] },
];

const CHANNEL_EQ_HI: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0752, data_offs: [0x0750, 0x0751, 0x0757, 0x0758], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0798, data_offs: [0x0793, 0x0794, 0x0799, 0x079A], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x07D7, data_offs: [0x07D3, 0x07D4, 0x07DA, 0x07DB], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0816, data_offs: [0x0814, 0x0815, 0x081B, 0x081C], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x085C, data_offs: [0x0857, 0x0858, 0x085D, 0x085E], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x089B, data_offs: [0x0897, 0x0898, 0x089E, 0x089F], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x08DA, data_offs: [0x08D8, 0x08D9, 0x08DF, 0x08E0], bit_indices: [3, 2, 4, 5] },
];

const CHANNEL_SEND_MON1: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0052, data_offs: [0x0050, 0x0051, 0x0053, 0x0054], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x0098, data_offs: [0x009B, 0x009C, 0x009D, 0x009E], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x00E5, data_offs: [0x00E6, 0x00E7, 0x00E8, 0x00E9], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0132, data_offs: [0x0130, 0x0131, 0x0133, 0x0134], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x0178, data_offs: [0x017B, 0x017C, 0x017D, 0x017E], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x01C5, data_offs: [0x01C6, 0x01C7, 0x01C8, 0x01C9], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0212, data_offs: [0x0210, 0x0211, 0x0213, 0x0214], bit_indices: [3, 2, 0, 1] },
];

const CHANNEL_SEND_MON2: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0052, data_offs: [0x0053, 0x0054, 0x0057, 0x0058], bit_indices: [0, 1, 4, 5] },
    FloatParam { msb_off: 0x0098, data_offs: [0x0096, 0x0097, 0x009D, 0x009E], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x00EC, data_offs: [0x00E8, 0x00E9, 0x00ED, 0x00EE], bit_indices: [5, 4, 0, 1] },
    FloatParam { msb_off: 0x0132, data_offs: [0x0133, 0x0134, 0x0137, 0x0138], bit_indices: [0, 1, 4, 5] },
    FloatParam { msb_off: 0x0178, data_offs: [0x0176, 0x0177, 0x017D, 0x017E], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x01CC, data_offs: [0x01C8, 0x01C9, 0x01CD, 0x01CE], bit_indices: [5, 4, 0, 1] },
    FloatParam { msb_off: 0x0212, data_offs: [0x0213, 0x0214, 0x0217, 0x0218], bit_indices: [0, 1, 4, 5] },
];

const CHANNEL_SEND_FX1: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0059, data_offs: [0x005A, 0x005B, 0x005C, 0x005D], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x00A6, data_offs: [0x00A4, 0x00A5, 0x00A7, 0x00A8], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x00EC, data_offs: [0x00EF, 0x00F0, 0x00F1, 0x00F2], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x0139, data_offs: [0x013A, 0x013B, 0x013C, 0x013D], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0186, data_offs: [0x0184, 0x0185, 0x0187, 0x0188], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x01CC, data_offs: [0x01CF, 0x01D0, 0x01D1, 0x01D2], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x0219, data_offs: [0x021A, 0x021B, 0x021C, 0x021D], bit_indices: [0, 1, 2, 3] },
];

const CHANNEL_SEND_FX2: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0060, data_offs: [0x005E, 0x005F, 0x0061, 0x0062], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x00A6, data_offs: [0x00A9, 0x00AA, 0x00AB, 0x00AC], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x00F3, data_offs: [0x00F4, 0x00F5, 0x00F6, 0x00F7], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0140, data_offs: [0x013E, 0x013F, 0x0141, 0x0142], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x0186, data_offs: [0x0189, 0x018A, 0x018B, 0x018C], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x01D3, data_offs: [0x01D4, 0x01D5, 0x01D6, 0x01D7], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0220, data_offs: [0x021E, 0x021F, 0x0221, 0x0222], bit_indices: [3, 2, 0, 1] },
];

// ── Bus parameters (5 buses: Main, Mon1, Mon2, FX1, FX2) ──────────────

const BUS_LEVELS: [FloatParam; 5] = [
    FloatParam { msb_off: 0x04C7, data_offs: [0x04C8, 0x04C9, 0x04CA, 0x04CB], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x0338, data_offs: [0x033B, 0x033C, 0x033D, 0x033E], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x0338, data_offs: [0x0336, 0x0337, 0x033D, 0x033E], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x03D2, data_offs: [0x03D0, 0x03D1, 0x03D3, 0x03D4], bit_indices: [3, 2, 0, 1] },
    FloatParam { msb_off: 0x0418, data_offs: [0x041B, 0x041C, 0x041D, 0x041E], bit_indices: [2, 3, 4, 5] },
];

const BUS_BALANCES: [FloatParam; 5] = [
    FloatParam { msb_off: 0x054C, data_offs: [0x0547, 0x0548, 0x0549, 0x054E], bit_indices: [6, 5, 4, 1] },
    FloatParam { msb_off: 0x0530, data_offs: [0x052E, 0x052F, 0x0531, 0x0536], bit_indices: [3, 2, 0, 5] },
    FloatParam { msb_off: 0x0536, data_offs: [0x0534, 0x0535, 0x0537, 0x053B], bit_indices: [3, 2, 0, 4] },
    FloatParam { msb_off: 0x053E, data_offs: [0x0539, 0x053A, 0x053B, 0x0540], bit_indices: [6, 5, 4, 1] },
    FloatParam { msb_off: 0x053E, data_offs: [0x053C, 0x053D, 0x053F, 0x0544], bit_indices: [3, 2, 0, 5] },
];

const BUS_LIMITERS: [FloatParam; 3] = [
    FloatParam { msb_off: 0x0B42, data_offs: [0x0B45, 0x0B46, 0x0B47, 0x0B48], bit_indices: [2, 3, 4, 5] },
    FloatParam { msb_off: 0x08FD, data_offs: [0x08FE, 0x08FF, 0x0900, 0x0901], bit_indices: [0, 1, 2, 3] },
    FloatParam { msb_off: 0x08FD, data_offs: [0x08F9, 0x08FA, 0x0900, 0x0901], bit_indices: [5, 4, 2, 3] },
];

// 9-band EQ per bus: Main(0), Mon1(1), Mon2(2), FX1(3), FX2(4)
// Mon2 shares SysEx memory with Mon1 — offsets valid but ranges may appear narrower
const NINE_BAND_62HZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B49, data_offs: [0x0B45, 0x0B46, 0x0B4C, 0x0B4D], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0904, data_offs: [0x08FF, 0x0900, 0x0905, 0x0906], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0904, data_offs: [0x08FF, 0x0900, 0x0901, 0x0906], bit_indices: [6, 5, 4, 1] },
    FloatParam { msb_off: 0x09EB, data_offs: [0x09E7, 0x09E8, 0x09EE, 0x09EF], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0A62, data_offs: [0x0A5D, 0x0A5E, 0x0A63, 0x0A64], bit_indices: [6, 5, 0, 1] },
];
const NINE_BAND_125HZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B50, data_offs: [0x0B4B, 0x0B4C, 0x0B51, 0x0B52], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0904, data_offs: [0x0902, 0x0903, 0x0909, 0x090A], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0904, data_offs: [0x0902, 0x0903, 0x0905, 0x090A], bit_indices: [3, 2, 0, 5] },
    FloatParam { msb_off: 0x09F2, data_offs: [0x09ED, 0x09EE, 0x09F3, 0x09F4], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0A62, data_offs: [0x0A60, 0x0A61, 0x0A67, 0x0A68], bit_indices: [3, 2, 4, 5] },
];
const NINE_BAND_250HZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B50, data_offs: [0x0B4E, 0x0B4F, 0x0B55, 0x0B56], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x090B, data_offs: [0x0907, 0x0908, 0x090E, 0x090F], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0909, data_offs: [0x0907, 0x0908, 0x090B, 0x090F], bit_indices: [3, 2, 1, 5] },
    FloatParam { msb_off: 0x09F2, data_offs: [0x09F0, 0x09F1, 0x09F7, 0x09F8], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0A69, data_offs: [0x0A65, 0x0A66, 0x0A6C, 0x0A6D], bit_indices: [5, 4, 2, 3] },
];
const NINE_BAND_500HZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B57, data_offs: [0x0B53, 0x0B54, 0x0B5A, 0x0B5B], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0912, data_offs: [0x090D, 0x090E, 0x0913, 0x0914], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x090E, data_offs: [0x090D, 0x090F, 0x0912, 0x0914], bit_indices: [2, 0, 3, 5] },
    FloatParam { msb_off: 0x09F9, data_offs: [0x09F5, 0x09F6, 0x09FC, 0x09FD], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0A70, data_offs: [0x0A6B, 0x0A6C, 0x0A71, 0x0A72], bit_indices: [6, 5, 0, 1] },
];
const NINE_BAND_1KHZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B5E, data_offs: [0x0B59, 0x0B5A, 0x0B5F, 0x0B60], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0912, data_offs: [0x0910, 0x0911, 0x0917, 0x0918], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x090E, data_offs: [0x090D, 0x090F, 0x0912, 0x0914], bit_indices: [2, 0, 3, 5] },
    FloatParam { msb_off: 0x0A00, data_offs: [0x09FB, 0x09FC, 0x0A01, 0x0A02], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0A70, data_offs: [0x0A6E, 0x0A6F, 0x0A75, 0x0A76], bit_indices: [3, 2, 4, 5] },
];
const NINE_BAND_2KHZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B5E, data_offs: [0x0B5C, 0x0B5D, 0x0B63, 0x0B64], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0919, data_offs: [0x0915, 0x0916, 0x091C, 0x091D], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0917, data_offs: [0x0915, 0x0916, 0x0919, 0x091D], bit_indices: [3, 2, 1, 5] },
    FloatParam { msb_off: 0x0A00, data_offs: [0x09FE, 0x09FF, 0x0A05, 0x0A06], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0A77, data_offs: [0x0A73, 0x0A74, 0x0A7A, 0x0A7B], bit_indices: [5, 4, 2, 3] },
];
const NINE_BAND_4KHZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B65, data_offs: [0x0B61, 0x0B62, 0x0B68, 0x0B69], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0920, data_offs: [0x091B, 0x091C, 0x0921, 0x0922], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x091C, data_offs: [0x091B, 0x091D, 0x0920, 0x0922], bit_indices: [2, 0, 3, 5] },
    FloatParam { msb_off: 0x0A07, data_offs: [0x0A03, 0x0A04, 0x0A0A, 0x0A0B], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0A7E, data_offs: [0x0A79, 0x0A7A, 0x0A7F, 0x0A80], bit_indices: [6, 5, 0, 1] },
];
const NINE_BAND_8KHZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B6C, data_offs: [0x0B67, 0x0B68, 0x0B6D, 0x0B6E], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0920, data_offs: [0x091E, 0x091F, 0x0925, 0x0926], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x091C, data_offs: [0x091B, 0x091D, 0x0920, 0x0922], bit_indices: [2, 0, 3, 5] },
    FloatParam { msb_off: 0x0A0E, data_offs: [0x0A09, 0x0A0A, 0x0A0F, 0x0A10], bit_indices: [6, 5, 0, 1] },
    FloatParam { msb_off: 0x0A7E, data_offs: [0x0A7C, 0x0A7D, 0x0A83, 0x0A84], bit_indices: [3, 2, 4, 5] },
];
const NINE_BAND_16KHZ: [FloatParam; 5] = [
    FloatParam { msb_off: 0x0B6C, data_offs: [0x0B6A, 0x0B6B, 0x0B71, 0x0B72], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0927, data_offs: [0x0923, 0x0924, 0x092A, 0x092B], bit_indices: [5, 4, 2, 3] },
    FloatParam { msb_off: 0x0925, data_offs: [0x0923, 0x0924, 0x0927, 0x092B], bit_indices: [3, 2, 1, 5] },
    FloatParam { msb_off: 0x0A0E, data_offs: [0x0A0C, 0x0A0D, 0x0A13, 0x0A14], bit_indices: [3, 2, 4, 5] },
    FloatParam { msb_off: 0x0A85, data_offs: [0x0A81, 0x0A82, 0x0A88, 0x0A89], bit_indices: [5, 4, 2, 3] },
];

// ── Boolean parameters (single byte 0x00/0x01) ─────────────────────────

struct BoolParam {
    offset: usize,
    inverted: bool,
}

const CHANNEL_MUTES: [BoolParam; 7] = [
    BoolParam { offset: 0x04CC, inverted: false },
    BoolParam { offset: 0x04CD, inverted: false },
    BoolParam { offset: 0x04CF, inverted: false },
    BoolParam { offset: 0x04D0, inverted: false },
    BoolParam { offset: 0x04D1, inverted: false },
    BoolParam { offset: 0x04D2, inverted: false },
    BoolParam { offset: 0x04D3, inverted: false },
];

const CHANNEL_SOLOS: [BoolParam; 7] = [
    BoolParam { offset: 0x04DF, inverted: false },
    BoolParam { offset: 0x04E0, inverted: false },
    BoolParam { offset: 0x04E1, inverted: false },
    BoolParam { offset: 0x04E2, inverted: false },
    BoolParam { offset: 0x04E4, inverted: false },
    BoolParam { offset: 0x04E5, inverted: false },
    BoolParam { offset: 0x04E6, inverted: false },
];

const CHANNEL_PHANTOMS: [BoolParam; 2] = [
    BoolParam { offset: 0x0737, inverted: false },
    BoolParam { offset: 0x0778, inverted: false },
];

// ── FX parameters (simple byte offsets, not FloatParam) ─────────────────

struct FxSysExLayout {
    param1_off: usize,
    param2_off: usize,
    preset_off: usize,
}

const FX_SYSEX: [FxSysExLayout; 2] = [
    FxSysExLayout { param1_off: 0x0BC5, param2_off: 0x0BC6, preset_off: 0x0BC9 },
    FxSysExLayout { param1_off: 0x0BCD, param2_off: 0x0BCF, preset_off: 0x0BD1 },
];

#[derive(Debug, Clone)]
pub struct SysExDump {
    pub raw: Vec<u8>,
}

pub fn validate_sysex_dump(data: &[u8]) -> Option<SysExDump> {
    if data.len() < MIN_DUMP_SIZE {
        log_warn!(
            "[SYSEX] Dump too small ({} bytes, expected >= {})",
            data.len(),
            MIN_DUMP_SIZE
        );
        return None;
    }

    if data[0] != SYSEX_START {
        log_warn!("[SYSEX] Missing SysEx start byte (got 0x{:02X})", data[0]);
        return None;
    }

    if data[data.len() - 1] != SYSEX_END {
        log_warn!(
            "[SYSEX] Missing SysEx end byte (got 0x{:02X})",
            data[data.len() - 1]
        );
        return None;
    }

    if data[1..4] != BEHRINGER_ID {
        log_warn!(
            "[SYSEX] Wrong manufacturer ID: {:02X} {:02X} {:02X}",
            data[1],
            data[2],
            data[3]
        );
        return None;
    }

    if data[4] != FLOW8_MODEL {
        log_warn!("[SYSEX] Wrong model byte (got 0x{:02X})", data[4]);
        return None;
    }

    log!(
        "[SYSEX] Valid FLOW 8 dump: {} bytes, header OK",
        data.len()
    );

    Some(SysExDump {
        raw: data.to_vec(),
    })
}

pub fn apply_dump_to_controller(dump: &SysExDump, controller: &mut FLOW8Controller) {

    controller.last_sysex_dump = Some(dump.raw.clone());

    #[cfg(any(debug_assertions, feature = "dev-tools"))]
    save_dump_to_file(&dump.raw);

    log_hex_dump(&dump.raw);

    let mut synced = 0usize;
    apply_channel_names(dump, controller);
    synced += apply_channel_levels(dump, controller);
    synced += apply_channel_gains(dump, controller);
    synced += apply_channel_pans(dump, controller);
    synced += apply_channel_compressors(dump, controller);
    synced += apply_channel_low_cuts(dump, controller);
    synced += apply_channel_eq(dump, controller);
    synced += apply_channel_sends(dump, controller);
    synced += apply_channel_mutes(dump, controller);
    synced += apply_channel_solos(dump, controller);
    synced += apply_channel_phantoms(dump, controller);
    synced += apply_bus_levels(dump, controller);
    synced += apply_bus_balances(dump, controller);
    synced += apply_bus_limiters(dump, controller);
    synced += apply_nine_band_eq(dump, controller);
    synced += apply_fx_params(dump, controller);

    log!(
        "[SYSEX] Dump applied ({} bytes). {} parameters synced.",
        dump.raw.len(),
        synced
    );
}

fn restore_sysex_byte(data: &[u8], pos: usize) -> Option<u8> {
    let group_pos = (pos + 2) % 7;
    if group_pos == 0 {
        return None;
    }
    if pos < group_pos {
        return Some(data[pos]);
    }
    let msb_off = pos - group_pos;
    if msb_off >= data.len() {
        return Some(data[pos]);
    }
    let msb = data[msb_off];
    let bit_index = group_pos - 1;
    let mut b = data[pos];
    if msb & (1 << bit_index) != 0 {
        b |= 0x80;
    }
    Some(b)
}

fn decode_sysex_name(data: &[u8], start: usize) -> String {
    let end = (start + NAME_SCAN_LEN).min(data.len());
    let mut restored: Vec<u8> = Vec::new();

    for i in start..end {
        if let Some(b) = restore_sysex_byte(data, i) {
            restored.push(b);
        }
    }

    let name_start = restored
        .iter()
        .position(|&b| b >= 0x20)
        .unwrap_or(restored.len());
    let after_start = &restored[name_start..];

    let name_len = after_start
        .iter()
        .position(|&b| b == 0x00)
        .unwrap_or(after_start.len());
    let raw_bytes = &after_start[..name_len];

    String::from_utf8(raw_bytes.to_vec())
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

fn apply_channel_names(dump: &SysExDump, controller: &mut FLOW8Controller) {
    let data = &dump.raw;
    for (i, ch) in controller.channels.iter_mut().enumerate() {
        let off = NAMES_START + i * NAMES_STRIDE;
        if off + NAME_SCAN_LEN > data.len() {
            continue;
        }
        let name = decode_sysex_name(data, off);
        if ch.name != name {
            log_debug!("[SYSEX] Channel {} name: \"{}\"", i, name);
        }
        ch.name = name;
        ch.name_synced = true;
    }
}

fn decode_float(data: &[u8], param: &FloatParam) -> Option<f32> {
    let max_off = *param.data_offs.iter().max()?;
    if param.msb_off >= data.len() || max_off >= data.len() {
        return None;
    }

    let msb = data[param.msb_off];
    let mut bytes = [0u8; 4];
    for i in 0..4 {
        let mut b = data[param.data_offs[i]];
        if msb & (1 << param.bit_indices[i]) != 0 {
            b |= 0x80;
        }
        bytes[i] = b;
    }

    let value = f32::from_le_bytes(bytes);
    if value.is_nan() || value.is_infinite() {
        return None;
    }
    Some(value)
}

fn db_to_cc(db: f32) -> u8 {
    if db < LEVEL_DB_MIN {
        return 0;
    }
    let clamped = db.clamp(LEVEL_DB_MIN, LEVEL_DB_MAX);
    let cc = 1.0 + ((clamped - LEVEL_DB_MIN) / (LEVEL_DB_MAX - LEVEL_DB_MIN)) * 126.0;
    cc.round().clamp(0.0, 127.0) as u8
}

fn range_to_cc(value: f32, min: f32, max: f32) -> u8 {
    let clamped = value.clamp(min, max);
    let normalized = (clamped - min) / (max - min);
    (normalized * 127.0).round() as u8
}

// ── Channel parameter apply functions ──────────────────────────────────

fn apply_channel_levels(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;
    for (i, param) in CHANNEL_LEVELS.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = db_to_cc(db);
            controller.channels[i].channel_strip.level = cc;
            controller.channels[i].channel_strip.level_synced = true;
            log_debug!("[SYSEX] Ch{} level: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }
    count
}

fn apply_channel_gains(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;
    for (i, param) in CHANNEL_GAINS.iter().enumerate() {
        if !controller.channels[i].has_gain() {
            continue;
        }
        if let Some(db) = decode_float(data, param) {
            let cc = range_to_cc(db, GAIN_MIN, GAIN_MAX);
            controller.channels[i].channel_strip.gain = cc;
            controller.channels[i].channel_strip.gain_synced = true;
            log_debug!("[SYSEX] Ch{} gain: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }
    count
}

fn apply_channel_pans(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;
    for (i, param) in CHANNEL_PANS.iter().enumerate() {
        if let Some(value) = decode_float(data, param) {
            let cc = range_to_cc(value, PAN_MIN, PAN_MAX);
            controller.channels[i].channel_strip.balance = cc;
            controller.channels[i].channel_strip.balance_synced = true;
            log_debug!("[SYSEX] Ch{} pan: {:.2} → CC {}", i + 1, value, cc);
            count += 1;
        }
    }
    count
}

fn apply_channel_compressors(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;
    for (i, param) in CHANNEL_COMPRESSORS.iter().enumerate() {
        if !controller.channels[i].has_compressor() {
            continue;
        }
        if let Some(value) = decode_float(data, param) {
            let cc = (value.clamp(COMP_MIN, COMP_MAX) * 100.0).round() as u8;
            controller.channels[i].channel_strip.compressor = cc;
            controller.channels[i].channel_strip.compressor_synced = true;
            log_debug!("[SYSEX] Ch{} comp: {:.2} → CC {}", i + 1, value, cc);
            count += 1;
        }
    }
    count
}

fn decode_lowcut(data: &[u8], param: &LowCutParam) -> Option<u16> {
    if param.lo_off >= data.len() || param.hi_off >= data.len() {
        return None;
    }
    Some(data[param.lo_off] as u16 + data[param.hi_off] as u16 * 128)
}

fn lowcut_hz_to_cc(hz: u16) -> u8 {
    let clamped = hz.clamp(LOWCUT_MIN_HZ, LOWCUT_MAX_HZ);
    let ratio = (clamped - LOWCUT_MIN_HZ) as f32 / (LOWCUT_MAX_HZ - LOWCUT_MIN_HZ) as f32;
    (ratio * 127.0).round() as u8
}

fn apply_channel_low_cuts(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;
    for (i, param) in CHANNEL_LOW_CUTS.iter().enumerate() {
        if !controller.channels[i].has_low_cut() {
            continue;
        }
        if let Some(hz) = decode_lowcut(data, param) {
            let cc = lowcut_hz_to_cc(hz);
            controller.channels[i].channel_strip.low_cut = cc;
            controller.channels[i].channel_strip.low_cut_synced = true;
            log_debug!("[SYSEX] Ch{} lowcut: {} Hz → CC {}", i + 1, hz, cc);
            count += 1;
        }
    }
    count
}

fn apply_channel_eq(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;

    for (i, param) in CHANNEL_EQ_LOW.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = range_to_cc(db, EQ_MIN, EQ_MAX);
            controller.channels[i].four_band_eq.low = cc;
            controller.channels[i].four_band_eq.low_synced = true;
            log_debug!("[SYSEX] Ch{} eq_low: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }
    for (i, param) in CHANNEL_EQ_LOW_MID.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = range_to_cc(db, EQ_MIN, EQ_MAX);
            controller.channels[i].four_band_eq.low_mid = cc;
            controller.channels[i].four_band_eq.low_mid_synced = true;
            log_debug!("[SYSEX] Ch{} eq_lmid: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }
    for (i, param) in CHANNEL_EQ_HI_MID.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = range_to_cc(db, EQ_MIN, EQ_MAX);
            controller.channels[i].four_band_eq.hi_mid = cc;
            controller.channels[i].four_band_eq.hi_mid_synced = true;
            log_debug!("[SYSEX] Ch{} eq_hmid: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }
    for (i, param) in CHANNEL_EQ_HI.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = range_to_cc(db, EQ_MIN, EQ_MAX);
            controller.channels[i].four_band_eq.hi = cc;
            controller.channels[i].four_band_eq.hi_synced = true;
            log_debug!("[SYSEX] Ch{} eq_hi: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }

    count
}

fn apply_channel_sends(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;

    for (i, param) in CHANNEL_SEND_MON1.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = db_to_cc(db);
            controller.channels[i].sends.mon1 = cc;
            controller.channels[i].sends.mon1_synced = true;
            log_debug!("[SYSEX] Ch{} send_mon1: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }
    for (i, param) in CHANNEL_SEND_MON2.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = db_to_cc(db);
            controller.channels[i].sends.mon2 = cc;
            controller.channels[i].sends.mon2_synced = true;
            log_debug!("[SYSEX] Ch{} send_mon2: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }
    for (i, param) in CHANNEL_SEND_FX1.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = db_to_cc(db);
            controller.channels[i].sends.fx1 = cc;
            controller.channels[i].sends.fx1_synced = true;
            log_debug!("[SYSEX] Ch{} send_fx1: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }
    for (i, param) in CHANNEL_SEND_FX2.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = db_to_cc(db);
            controller.channels[i].sends.fx2 = cc;
            controller.channels[i].sends.fx2_synced = true;
            log_debug!("[SYSEX] Ch{} send_fx2: {:.1} dB → CC {}", i + 1, db, cc);
            count += 1;
        }
    }

    count
}

fn apply_channel_mutes(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;
    for (i, param) in CHANNEL_MUTES.iter().enumerate() {
        if param.offset >= data.len() {
            continue;
        }
        let raw = data[param.offset];
        let is_muted = if param.inverted { raw == 0x00 } else { raw != 0x00 };
        controller.channels[i].is_muted = is_muted;
        controller.channels[i].mute_synced = true;
        log_debug!("[SYSEX] Ch{} mute: byte=0x{:02X} → {}", i + 1, raw, is_muted);
        count += 1;
    }
    count
}

fn apply_channel_solos(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;
    for (i, param) in CHANNEL_SOLOS.iter().enumerate() {
        if param.offset >= data.len() {
            continue;
        }
        let raw = data[param.offset];
        let is_soloed = if param.inverted { raw == 0x00 } else { raw != 0x00 };
        controller.channels[i].is_soloed = is_soloed;
        controller.channels[i].solo_synced = true;
        log_debug!("[SYSEX] Ch{} solo: byte=0x{:02X} → {}", i + 1, raw, is_soloed);
        count += 1;
    }
    count
}

fn apply_channel_phantoms(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let mut count = 0;
    for (i, param) in CHANNEL_PHANTOMS.iter().enumerate() {
        if param.offset >= data.len() {
            continue;
        }
        let raw = data[param.offset];
        let is_on = if param.inverted { raw == 0x00 } else { raw != 0x00 };
        controller.channels[i].phantom_pwr.is_on = is_on;
        controller.channels[i].phantom_synced = true;
        log_debug!("[SYSEX] Ch{} phantom: byte=0x{:02X} → {}", i + 1, raw, is_on);
        count += 1;
    }
    count
}

// ── Bus parameter apply functions ──────────────────────────────────────

fn apply_bus_levels(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let bus_names = ["Main", "Mon1", "Mon2", "FX1", "FX2"];
    let mut count = 0;
    for (i, param) in BUS_LEVELS.iter().enumerate() {
        if let Some(db) = decode_float(data, param) {
            let cc = db_to_cc(db);
            controller.buses[i].bus_strip.level = cc;
            controller.buses[i].bus_strip.level_synced = true;
            log_debug!("[SYSEX] {} level: {:.1} dB → CC {}", bus_names[i], db, cc);
            count += 1;
        }
    }
    count
}

fn apply_bus_balances(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let bus_names = ["Main", "Mon1", "Mon2", "FX1", "FX2"];
    let mut count = 0;
    for (i, param) in BUS_BALANCES.iter().enumerate() {
        if !controller.buses[i].has_balance() {
            continue;
        }
        if let Some(value) = decode_float(data, param) {
            let cc = range_to_cc(value, BAL_MIN, BAL_MAX);
            controller.buses[i].bus_strip.balance = cc;
            controller.buses[i].bus_strip.balance_synced = true;
            log_debug!("[SYSEX] {} balance: {:.2} → CC {}", bus_names[i], value, cc);
            count += 1;
        }
    }
    count
}

fn apply_bus_limiters(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let bus_names = ["Main", "Mon1", "Mon2", "FX1", "FX2"];
    let mut count = 0;
    for (i, param) in BUS_LIMITERS.iter().enumerate() {
        if !controller.buses[i].has_limiter() {
            continue;
        }
        if let Some(value) = decode_float(data, param) {
            let cc = range_to_cc(value, LIMITER_MIN, LIMITER_MAX);
            controller.buses[i].bus_strip.limiter = cc;
            controller.buses[i].bus_strip.limiter_synced = true;
            log_debug!("[SYSEX] {} limiter: {:.1} → CC {}", bus_names[i], value, cc);
            count += 1;
        }
    }
    count
}

fn apply_nine_band_eq(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let bus_names = ["Main", "Mon1", "Mon2", "FX1", "FX2"];
    let band_names = ["62Hz", "125Hz", "250Hz", "500Hz", "1kHz", "2kHz", "4kHz", "8kHz", "16kHz"];
    let bands: [&[FloatParam; 5]; 9] = [
        &NINE_BAND_62HZ, &NINE_BAND_125HZ, &NINE_BAND_250HZ,
        &NINE_BAND_500HZ, &NINE_BAND_1KHZ, &NINE_BAND_2KHZ,
        &NINE_BAND_4KHZ, &NINE_BAND_8KHZ, &NINE_BAND_16KHZ,
    ];
    let mut count = 0;

    for (band_idx, band_params) in bands.iter().enumerate() {
        for (bus_idx, param) in band_params.iter().enumerate() {
            if !controller.buses[bus_idx].has_nine_band_eq() {
                continue;
            }
            if let Some(value) = decode_float(data, param) {
                let cc = range_to_cc(value, EQ_MIN, EQ_MAX);
                let eq = &mut controller.buses[bus_idx].nine_band_eq;
                match band_idx {
                    0 => eq.freq_62_hz = cc,
                    1 => eq.freq_125_hz = cc,
                    2 => eq.freq_250_hz = cc,
                    3 => eq.freq_500_hz = cc,
                    4 => eq.freq_1_khz = cc,
                    5 => eq.freq_2_khz = cc,
                    6 => eq.freq_4_khz = cc,
                    7 => eq.freq_8_khz = cc,
                    8 => eq.freq_16_khz = cc,
                    _ => unreachable!(),
                }
                eq.bands_synced[band_idx] = true;
                log_debug!(
                    "[SYSEX] {} {}: {:.1} dB → CC {}",
                    bus_names[bus_idx], band_names[band_idx], value, cc
                );
                count += 1;
            }
        }
    }

    count
}

fn apply_fx_params(dump: &SysExDump, controller: &mut FLOW8Controller) -> usize {
    let data = &dump.raw;
    let fx_names = ["FX1", "FX2"];
    let mut count = 0;

    for (i, layout) in FX_SYSEX.iter().enumerate() {
        if layout.preset_off >= data.len() {
            continue;
        }

        let preset_raw = data[layout.preset_off];
        if preset_raw > 0 && preset_raw <= 16 {
            controller.fx_slots[i].preset = preset_raw - 1;
            controller.fx_slots[i].preset_synced = true;
            log_debug!("[SYSEX] {} preset: {} (raw {})", fx_names[i], preset_raw - 1, preset_raw);
            count += 1;
        }

        let param1_byte = data[layout.param1_off];
        let param1_cc = (param1_byte as u16 * 127 / 100).min(127) as u8;
        controller.fx_slots[i].param1 = param1_cc;
        controller.fx_slots[i].param1_synced = true;
        log_debug!("[SYSEX] {} param1: byte {} → CC {}", fx_names[i], param1_byte, param1_cc);
        count += 1;

        let param2_byte = data[layout.param2_off];
        let param2_cc = (param2_byte as u16 * 127 / 100).min(127) as u8;
        controller.fx_slots[i].param2 = param2_cc;
        controller.fx_slots[i].param2_synced = true;
        log_debug!("[SYSEX] {} param2: byte {} → CC {}", fx_names[i], param2_byte, param2_cc);
        count += 1;
    }

    count
}

#[cfg(any(debug_assertions, feature = "dev-tools"))]
fn save_dump_to_file(data: &[u8]) {
    let dumps_dir = PathBuf::from("calibration-data");
    if let Err(e) = fs::create_dir_all(&dumps_dir) {
        log_warn!("[SYSEX] Failed to create dumps dir: {}", e);
        return;
    }

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = format!("sysex-dump-{}.hex", timestamp);
    let path = dumps_dir.join(&filename);

    let hex_content = format_hex_dump(data);
    match fs::write(&path, &hex_content) {
        Ok(_) => log!("[SYSEX] Dump saved to {}", path.display()),
        Err(e) => log_warn!("[SYSEX] Failed to save dump: {}", e),
    }
}

fn log_hex_dump(data: &[u8]) {
    let head = 128.min(data.len());
    log!("[SYSEX] Hex dump (first {} of {} bytes):", head, data.len());

    for (i, chunk) in data[..head].chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if (0x20..=0x7E).contains(&b) { b as char } else { '.' })
            .collect();
        log_debug!(
            "[SYSEX]   {:04X}: {:48} | {}",
            i * 16,
            hex.join(" "),
            ascii
        );
    }

    if data.len() > 128 {
        let tail_start = data.len().saturating_sub(32);
        log!("[SYSEX] ... (tail, last {} bytes):", data.len() - tail_start);
        for (i, chunk) in data[tail_start..].chunks(16).enumerate() {
            let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
            let ascii: String = chunk
                .iter()
                .map(|&b| if (0x20..=0x7E).contains(&b) { b as char } else { '.' })
                .collect();
            log_debug!(
                "[SYSEX]   {:04X}: {:48} | {}",
                tail_start + i * 16,
                hex.join(" "),
                ascii
            );
        }
    }
}

pub fn format_hex_dump(data: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if (0x20..=0x7E).contains(&b) { b as char } else { '.' })
            .collect();
        out.push_str(&format!("{:04X}: {:48} | {}\n", i * 16, hex.join(" "), ascii));
    }
    out
}
