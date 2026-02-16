mod protocol;
mod ant_driver;
mod replay_driver; // Don't forget to declare the new module

#[tauri::command]
fn start_listening(app: tauri::AppHandle, port: String) -> String {
    // TRICK: If the user types "replay" or "test" into the Port box, 
    // run the simulation instead of the USB driver.
    if port.to_lowercase() == "replay" {
        let player = replay_driver::ReplayDriver::new(app);
        player.start();
        return "Replaying capture.jsonl...".to_string();
    }

    // Otherwise, start the real hardware driver
    let driver = ant_driver::AntDriver::new(port, app);
    match driver.start() {
        Ok(_) => "Listening on USB...".to_string(),
        Err(e) => format!("Error: {}", e),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![start_listening])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}