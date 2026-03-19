use crate::service::midi::{send_cc, send_note_on, send_program_change};
use crate::model::message::InterfaceMessage;
use midir::MidiOutputConnection;

/// Maps InterfaceMessage events to MIDI commands.
///
/// MIDI channel assignment (0-indexed):
///   Input Ch. 1-7 → 0-6
///   Main Bus → 7, Mon1 → 8, Mon2 → 9, FX1 Bus → 10, FX2 Bus → 11
///   FX1 Control → 13, FX2 Control → 14
///   Global → 15 (snapshots, FX mute, tap tempo)
pub fn match_midi_command(
    message: &InterfaceMessage,
    midi_conn: &mut Option<MidiOutputConnection>,
) {
    let conn = match midi_conn.as_mut() {
        Some(c) => c,
        None => return,
    };

    match *message {
        InterfaceMessage::NavigateTo(_)
        | InterfaceMessage::RetryConnection
        | InterfaceMessage::Disconnect
        | InterfaceMessage::ThemeChanged(_)
        | InterfaceMessage::OpenManual
        | InterfaceMessage::OpenRepository
        | InterfaceMessage::OpenDonation
        | InterfaceMessage::CopyDebugLog
        | InterfaceMessage::SaveDebugLog
        | InterfaceMessage::CloseToTrayChanged(_)
        | InterfaceMessage::Tick
        | InterfaceMessage::WindowCloseRequested(_)
        | InterfaceMessage::MainWindowIdResolved(_)
        | InterfaceMessage::TrayEvent(_)
        | InterfaceMessage::BleConnect
        | InterfaceMessage::BleRequestDump
        | InterfaceMessage::SyncIntervalChanged(_) => {}

        #[cfg(any(debug_assertions, feature = "dev-tools"))]
        InterfaceMessage::CopyHexDump => {}
        #[cfg(any(debug_assertions, feature = "dev-tools"))]
        InterfaceMessage::CalibrateStart => {}
        #[cfg(any(debug_assertions, feature = "dev-tools"))]
        InterfaceMessage::DigestRun => {}

        // --- Input Channels (MIDI Ch = channel_id) ---
        InterfaceMessage::Mute(ch, value) => send_cc(conn, ch, 5, if !value { 127 } else { 0 }),
        InterfaceMessage::Solo(ch, value) => send_cc(conn, ch, 6, if !value { 127 } else { 0 }),
        InterfaceMessage::Gain(ch, value) => send_cc(conn, ch, 8, value),
        InterfaceMessage::Level(ch, value) => send_cc(conn, ch, 7, value),
        InterfaceMessage::Balance(ch, value) => send_cc(conn, ch, 10, value),
        InterfaceMessage::LowCut(ch, value) => send_cc(conn, ch, 9, value),
        InterfaceMessage::Compressor(ch, value) => send_cc(conn, ch, 11, value),
        InterfaceMessage::PhantomPower(_) => {}

        // --- Channel EQ ---
        InterfaceMessage::EqLow(ch, value) => send_cc(conn, ch, 1, value),
        InterfaceMessage::EqLowMid(ch, value) => send_cc(conn, ch, 2, value),
        InterfaceMessage::EqHiMid(ch, value) => send_cc(conn, ch, 3, value),
        InterfaceMessage::EqHi(ch, value) => send_cc(conn, ch, 4, value),

        // --- Channel Sends ---
        InterfaceMessage::SendMon1(ch, value) => send_cc(conn, ch, 14, value),
        InterfaceMessage::SendMon2(ch, value) => send_cc(conn, ch, 15, value),
        InterfaceMessage::SendFx1(ch, value) => send_cc(conn, ch, 16, value),
        InterfaceMessage::SendFx2(ch, value) => send_cc(conn, ch, 17, value),

        // --- Buses (MIDI Ch = bus_id: Main=7, Mon1=8, Mon2=9, FX1=10, FX2=11) ---
        InterfaceMessage::BusLevel(_, bus_id, value) => send_cc(conn, bus_id, 7, value),
        InterfaceMessage::BusBalance(_, bus_id, value) => send_cc(conn, bus_id, 10, value),
        InterfaceMessage::BusLimiter(_, bus_id, value) => send_cc(conn, bus_id, 8, value),
        InterfaceMessage::BusNineBandEq(_, bus_id, band_index, value) => {
            send_cc(conn, bus_id, 11 + band_index, value)
        }

        // --- FX Control (FX1 → MIDI Ch 13, FX2 → MIDI Ch 14) ---
        InterfaceMessage::FxPreset(fx_id, preset) => {
            send_program_change(conn, 13 + fx_id, preset + 1)
        }
        InterfaceMessage::FxParam1(fx_id, value) => send_cc(conn, 13 + fx_id, 1, value),
        InterfaceMessage::FxParam2(fx_id, value) => send_cc(conn, 13 + fx_id, 2, value),

        // --- Global (MIDI Ch 15 = Ch 16 in 1-indexed) ---
        InterfaceMessage::FxMute => {}
        InterfaceMessage::TapTempo => send_note_on(conn, 15, 0, 127),
        InterfaceMessage::LoadSnapshot(n) => send_program_change(conn, 15, n + 1),
        InterfaceMessage::ResetMixer => send_program_change(conn, 15, 16),
    }
}
