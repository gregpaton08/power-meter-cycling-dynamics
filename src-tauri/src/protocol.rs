use serde::{Deserialize, Serialize};

// ANT+ Constants
pub const SYNC_BYTE: u8 = 0xA4;
pub const MSG_BROADCAST_DATA: u8 = 0x4E;

// Public ANT+ Network Key (Standard for all sport devices)
pub const ANT_PLUS_NET_KEY: [u8; 8] = [0xB9, 0xA5, 0x21, 0xFB, 0xBD, 0x72, 0xC3, 0x45];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CyclingData {
    pub instant_power: u16,
    pub cadence: u8,
    pub l_torque_eff: u8,
    pub r_torque_eff: u8,
    pub l_pedal_smooth: u8,
    pub r_pedal_smooth: u8,
    pub l_phase_start: f32, // Degrees
    pub l_phase_end: f32,   // Degrees
    pub r_phase_start: f32, // Degrees
    pub r_phase_end: f32,   // Degrees
    pub pco: i8,            // Platform Center Offset (mm)
}

pub fn parse_page_10(payload: &[u8], state: &mut CyclingData) {
    // Standard Power Page (0x10)
    // Byte 3: Cadence
    state.cadence = payload[3];

    // Byte 6-7: Instant Power (Little Endian)
    let low = payload[6] as u16;
    let high = payload[7] as u16;
    state.instant_power = low | (high << 8);
}

pub fn parse_page_13(payload: &[u8], state: &mut CyclingData) {
    // Torque Effectiveness & Pedal Smoothness (0x13)
    // Left: Byte 1 (TE), Byte 3 (PS)
    // Right: Byte 2 (TE), Byte 4 (PS)
    // Values are 0-100 (0.5% steps). 0xFF is invalid.

    if payload[1] != 0xFF {
        state.l_torque_eff = payload[1] / 2;
    }
    if payload[2] != 0xFF {
        state.r_torque_eff = payload[2] / 2;
    }
    if payload[3] != 0xFF {
        state.l_pedal_smooth = payload[3] / 2;
    }
    if payload[4] != 0xFF {
        state.r_pedal_smooth = payload[4] / 2;
    }
}

pub fn parse_page_19(payload: &[u8], state: &mut CyclingData) {
    // Torque Effectiveness / Cycling Dynamics (0x19)
    // This often contains Power Phase.
    // NOTE: This page is complex and has sub-pages defined by Byte 1.
    // For now, let's just parse the standard Power Phase if present.

    // Byte 2: Power Phase Start L (1/256 of circle)
    // Byte 3: Power Phase Length L
    let l_start = payload[2] as f32;
    let l_len = payload[3] as f32;

    if l_start != 0.0 {
        // Filter zeros/invalid
        state.l_phase_start = (l_start / 256.0) * 360.0;
        let end_raw = l_start + l_len;
        state.l_phase_end = ((end_raw / 256.0) * 360.0) % 360.0;
    }

    let r_start = payload[4] as f32;
    let r_len = payload[5] as f32;
    if r_start != 0.0 {
        state.r_phase_start = (r_start / 256.0) * 360.0;
        let end_raw = r_start + r_len;
        state.r_phase_end = ((end_raw / 256.0) * 360.0) % 360.0;
    }
}
