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

// Helper to decode "Power Only" Page 0x10
pub fn parse_page_10(payload: &[u8], state: &mut CyclingData) {
    // Byte 6: Instant Cadence
    state.cadence = payload[6];
    // Byte 7: Instant Power LSB, combined with MSB logic if needed (simplified here)
    state.instant_power = (payload[7] as u16) + ((payload[6] as u16) << 8); 
    // Note: Standard accumulates power, but for Favero often payload[7] + low nibble of 6 is instant.
    // Simplified: Just taking byte 7 + 8 for generic power meters often works differently.
    // Better implementation for standard Power Profile:
    state.instant_power = (payload[7] as u16) | ((payload[6] as u16) << 8);
}

// Helper to decode "Torque Effectiveness & Pedal Smoothness" Page 0x13
pub fn parse_page_13(payload: &[u8], state: &mut CyclingData) {
    // Byte 1: Left Torque Effectiveness (0-100, 0xFF=Invalid)
    if payload[1] != 0xFF { state.l_torque_eff = payload[1] / 2; }
    // Byte 2: Right Torque Effectiveness
    if payload[2] != 0xFF { state.r_torque_eff = payload[2] / 2; }
    // Byte 3: Left Pedal Smoothness
    if payload[3] != 0xFF { state.l_pedal_smooth = payload[3] / 2; }
    // Byte 4: Right Pedal Smoothness
    if payload[4] != 0xFF { state.r_pedal_smooth = payload[4] / 2; }
}

// Helper to decode "Cycling Dynamics" Page 0x19 (Power Phase)
// This is complex. Favero sends this interleaved.
pub fn parse_page_19(payload: &[u8], state: &mut CyclingData) {
    // Subpage identifier is often in Byte 1 or implicit logic
    // Simplified Parsing for Standard ANT+ Power Phase
    // Byte 2: Power Phase Start (Left) - 1/256 steps of 360 degrees
    // Byte 3: Power Phase Length (Left)
    // Byte 4: Power Phase Start (Right)
    // Byte 5: Power Phase Length (Right)
    
    let l_start_val = payload[2] as f32;
    let l_len_val = payload[3] as f32;
    state.l_phase_start = (l_start_val / 256.0) * 360.0;
    let l_end_raw = l_start_val + l_len_val;
    state.l_phase_end = ((l_end_raw / 256.0) * 360.0) % 360.0;

    let r_start_val = payload[4] as f32;
    let r_len_val = payload[5] as f32;
    state.r_phase_start = (r_start_val / 256.0) * 360.0;
    let r_end_raw = r_start_val + r_len_val;
    state.r_phase_end = ((r_end_raw / 256.0) * 360.0) % 360.0;
}
