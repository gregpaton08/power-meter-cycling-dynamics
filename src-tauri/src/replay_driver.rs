use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use serde::Deserialize;
use crate::protocol::{self, CyclingData};

#[derive(Deserialize)]
struct PacketLog {
    ts: u128,
    data: Vec<u8>,
}

pub struct ReplayDriver {
    app: AppHandle,
    state: Arc<Mutex<CyclingData>>,
}

impl ReplayDriver {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            state: Arc::new(Mutex::new(CyclingData::default())),
        }
    }

    pub fn start(&self) {
        let app_handle = self.app.clone();
        let state_clone = self.state.clone();

        thread::spawn(move || {
            let file = match File::open("capture.jsonl") {
                Ok(f) => f,
                Err(_) => {
                    eprintln!("No capture file found!");
                    return;
                }
            };
            
            let reader = BufReader::new(file);
            let mut last_ts = 0;

            println!("Starting Replay...");

            for line in reader.lines() {
                if let Ok(json_str) = line {
                    if let Ok(record) = serde_json::from_str::<PacketLog>(&json_str) {
                        // 1. Timing Simulation
                        if last_ts > 0 {
                            let delta = record.ts.saturating_sub(last_ts);
                            // Cap delay to avoid huge freezes if capture had gaps
                            let sleep_ms = if delta > 2000 { 1000 } else { delta as u64 };
                            thread::sleep(Duration::from_millis(sleep_ms));
                        }
                        last_ts = record.ts;

                        // 2. Parse exactly like the Live Driver
                        let mut state = state_clone.lock().unwrap();
                        let payload = &record.data;
                        let page = payload[0] & 0x7F;

                        match page {
                            0x10 => protocol::parse_page_10(payload, &mut state),
                            0x13 => protocol::parse_page_13(payload, &mut state),
                            0x19 => protocol::parse_page_19(payload, &mut state),
                            _ => {}
                        }

                        // 3. Emit to UI
                        let _ = app_handle.emit("cycling-data", &*state);
                    }
                }
            }
            println!("Replay Finished.");
        });
    }
}
