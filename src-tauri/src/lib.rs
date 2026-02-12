use tauri::AppHandle;
mod protocol;
mod ant_driver;

#[tauri::command]
fn start_listening(app: AppHandle, port: String) -> String {
    let driver = ant_driver::AntDriver::new(port.clone(), app);
    match driver.start() {
        Ok(_) => format!("Listening on {}", port),
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