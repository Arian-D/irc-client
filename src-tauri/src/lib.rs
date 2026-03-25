// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn send(user: &str, message: &str) -> String {
    dbg!("Sent the message {message} as {user}!");
    format!("{user}: {message}")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![send])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
