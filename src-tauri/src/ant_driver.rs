use std::io::{Read, Write};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use serialport::SerialPort;
use tauri::{AppHandle, Emitter};
use crate::protocol::{self, CyclingData, SYNC_BYTE, ANT_PLUS_NET_KEY};

pub struct AntDriver {
    port_name: String,
    app: AppHandle,
    state: Arc<Mutex<CyclingData>>,
}

impl AntDriver {
    pub fn new(port_name: String, app: AppHandle) -> Self {
        Self {
            port_name,
            app,
            state: Arc::new(Mutex::new(CyclingData::default())),
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut port = serialport::new(&self.port_name, 115_200)
            .timeout(Duration::from_millis(1000))
            .open()?;

        // 1. Reset System
        self.send_msg(&mut port, 0x4A, &[0x00])?; 
        std::thread::sleep(Duration::from_millis(500));

        // 2. Set Network Key (Net 0)
        let mut net_msg = vec![0x00]; // Net Number 0
        net_msg.extend_from_slice(&ANT_PLUS_NET_KEY);
        self.send_msg(&mut port, 0x46, &net_msg)?;

        // 3. Assign Channel (Channel 0, Type 0x00 = Slave/Receive)
        self.send_msg(&mut port, 0x42, &[0x00, 0x00, 0x00])?;

        // 4. Set Channel ID (Chan 0, Device 0(Wildcard), Type 0(Wildcard), Manuf 0(Wildcard))
        self.send_msg(&mut port, 0x51, &[0x00, 0x00, 0x00, 0x00, 0x00])?;

        // 5. Set Radio Freq (Chan 0, Freq 57 = 2457MHz)
        self.send_msg(&mut port, 0x45, &[0x00, 57])?;

        // 6. Set Period (Chan 0, Period 8182 = ~4Hz for Power Meters)
        // 8182 in Little Endian = [0xF6, 0x1F]
        self.send_msg(&mut port, 0x43, &[0x00, 0xF6, 0x1F])?;

        // 7. Open Channel (Chan 0)
        self.send_msg(&mut port, 0x4B, &[0x00])?;
        
        println!("ANT+ Channel Opened. Listening...");

        // READ LOOP
        let mut buf = [0u8; 1024];
        let state_clone = self.state.clone();
        let app_handle = self.app.clone();

        std::thread::spawn(move || {
            loop {
                // Read chunks
                match port.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        // Simple parser: iterate through buffer to find SYNC
                        for i in 0..n {
                            if buf[i] == SYNC_BYTE && i + 1 < n {
                                let len = buf[i+1] as usize;
                                if i + 3 + len <= n {
                                    let msg_id = buf[i+2];
                                    let data = &buf[i+3..i+3+len];
                                    
                                    // Process Broadcast Data (0x4E)
                                    if msg_id == protocol::MSG_BROADCAST_DATA {
                                        let mut state = state_clone.lock().unwrap();
                                        
                                        // data[0] is Channel Num
                                        // data[1..9] is the 8-byte Payload
                                        let payload = &data[1..9];
                                        let page = payload[0] & 0x7F; // Mask out toggle bit

                                        match page {
                                            0x10 => protocol::parse_page_10(payload, &mut state),
                                            0x13 => protocol::parse_page_13(payload, &mut state),
                                            0x19 => protocol::parse_page_19(payload, &mut state),
                                            _ => {} // Ignore other pages
                                        }

                                        // Emit to UI
                                        let _ = app_handle.emit("cycling-data", &*state);
                                    }
                                }
                            }
                        }
                    }
                    Ok(_) => {},
                    Err(e) => eprintln!("Read error: {:?}", e),
                }
            }
        });

        Ok(())
    }

    fn send_msg(&self, port: &mut Box<dyn SerialPort>, msg_id: u8, data: &[u8]) -> Result<(), std::io::Error> {
        let len = data.len() as u8;
        let mut buf = vec![SYNC_BYTE, len, msg_id];
        buf.extend_from_slice(data);
        
        // Checksum (XOR of all bytes)
        let mut checksum = 0;
        for b in &buf { checksum ^= b; }
        buf.push(checksum);

        port.write_all(&buf)?;
        Ok(())
    }
}
