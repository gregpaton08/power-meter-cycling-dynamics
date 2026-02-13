use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};
use rusb::{Context, Device, DeviceHandle, Direction, TransferType, UsbContext};
use crate::protocol::{self, CyclingData, SYNC_BYTE, ANT_PLUS_NET_KEY};

// Common Vendor IDs
const VID_GARMIN: u16 = 0x0fcf; 
const VID_SILABS: u16 = 0x10c4; 

pub struct AntDriver {
    app: AppHandle,
    state: Arc<Mutex<CyclingData>>,
}

impl AntDriver {
    pub fn new(_port_name: String, app: AppHandle) -> Self {
        Self {
            app,
            state: Arc::new(Mutex::new(CyclingData::default())),
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let context = Context::new()?;
        
        // 1. Find Device
        let (device, mut handle, ep_in, ep_out) = self.find_ant_device(&context)
            .ok_or("No ANT+ Stick found.")?;

        println!("Found Stick: Bus {:03} Device {:03}", device.bus_number(), device.address());

        // 2. THE FIX: Force Reset the Device
        // This simulates unplugging/replugging it, which kicks macOS off the driver.
        // Note: This might cause the 'handle' to become invalid on some systems,
        // so we might need to re-open it if this fails.
        println!("Resetting USB Device to kick off macOS...");
        if let Err(e) = handle.reset() {
            eprintln!("Warning: Device reset failed (might be fine): {}", e);
        }
        
        // Give the OS a moment to realize it's gone
        thread::sleep(Duration::from_millis(1000));

        // 3. Detach Kernel (If reset didn't work)
        let interface_num = 0;
        if let Ok(active) = handle.kernel_driver_active(interface_num) {
            if active {
                println!("Kernel driver active. Force detaching...");
                let _ = handle.detach_kernel_driver(interface_num);
            }
        }

        // 4. Claim Interface
        handle.claim_interface(interface_num)?;
        println!("Interface Claimed!");

        // 5. Initialize ANT+
        // Standard Garmin Stick 2 does NOT want padding. 
        self.send_usb_msg(&handle, ep_out, 0x4A, &[0x00])?; // Reset System
        thread::sleep(Duration::from_millis(500));

        let mut net_msg = vec![0x00]; 
        net_msg.extend_from_slice(&ANT_PLUS_NET_KEY);
        self.send_usb_msg(&handle, ep_out, 0x46, &net_msg)?; // Set Network Key
        self.send_usb_msg(&handle, ep_out, 0x42, &[0x00, 0x00, 0x00])?; // Assign Chan 0
        self.send_usb_msg(&handle, ep_out, 0x51, &[0x00, 0x00, 0x00, 0x00, 0x00])?; // ID
        self.send_usb_msg(&handle, ep_out, 0x45, &[0x00, 57])?; // Freq
        self.send_usb_msg(&handle, ep_out, 0x43, &[0x00, 0xF6, 0x1F])?; // Period
        self.send_usb_msg(&handle, ep_out, 0x4B, &[0x00])?; // Open Channel

        println!("ANT+ Initialized. Reading...");

        // 6. Read Loop
        let state_clone = self.state.clone();
        let app_handle = self.app.clone();
        
        thread::spawn(move || {
            let mut buf = [0u8; 64];
            loop {
                // We use a short timeout so we can keep looping
                match handle.read_bulk(ep_in, &mut buf, Duration::from_millis(100)) {
                    Ok(n) if n > 0 => {
                        for i in 0..n {
                            if buf[i] == SYNC_BYTE && i + 1 < n {
                                let len = buf[i+1] as usize;
                                if i + 3 + len <= n {
                                    let msg_id = buf[i+2];
                                    let data = &buf[i+3..i+3+len];
                                    if msg_id == protocol::MSG_BROADCAST_DATA {
                                        let mut state = state_clone.lock().unwrap();
                                        let payload = &data[1..9]; 
                                        let page = payload[0] & 0x7F;
                                        match page {
                                            0x10 => protocol::parse_page_10(payload, &mut state),
                                            0x13 => protocol::parse_page_13(payload, &mut state),
                                            0x19 => protocol::parse_page_19(payload, &mut state),
                                            _ => {} 
                                        }
                                        let _ = app_handle.emit("cycling-data", &*state);
                                    }
                                }
                            }
                        }
                    },
                    Ok(_) => {}, 
                    Err(rusb::Error::Timeout) => {}, // Ignore timeouts
                    Err(e) => {
                        // If we get an Input/Output error, the device might have disconnected
                        eprintln!("Read Error: {:?}", e);
                        thread::sleep(Duration::from_millis(1000));
                    }
                }
            }
        });

        Ok(())
    }

    fn find_ant_device(&self, context: &Context) -> Option<(Device<Context>, DeviceHandle<Context>, u8, u8)> {
        for device in context.devices().ok()?.iter() {
            let desc = device.device_descriptor().ok()?;
            if desc.vendor_id() == VID_GARMIN || desc.vendor_id() == VID_SILABS {
                let config = device.active_config_descriptor().ok()?;
                let mut ep_in = None;
                let mut ep_out = None;

                for interface in config.interfaces() {
                    for descriptor in interface.descriptors() {
                        for endpoint in descriptor.endpoint_descriptors() {
                            if endpoint.transfer_type() == TransferType::Bulk {
                                if endpoint.direction() == Direction::In { ep_in = Some(endpoint.address()); }
                                else { ep_out = Some(endpoint.address()); }
                            }
                        }
                    }
                }
                if let (Some(i), Some(o)) = (ep_in, ep_out) {
                    if let Ok(handle) = device.open() {
                        return Some((device, handle, i, o));
                    }
                }
            }
        }
        None
    }

    fn send_usb_msg(&self, handle: &DeviceHandle<Context>, ep_out: u8, msg_id: u8, data: &[u8]) -> Result<(), rusb::Error> {
        let len = data.len() as u8;
        let mut buf = vec![SYNC_BYTE, len, msg_id];
        buf.extend_from_slice(data);
        let mut checksum = 0;
        for b in &buf { checksum ^= b; }
        buf.push(checksum);
        
        // NO PADDING for Garmin Stick 2
        handle.write_bulk(ep_out, &buf, Duration::from_millis(1000)).map(|_| ())
    }
}