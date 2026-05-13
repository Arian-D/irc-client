use log::{debug, info};
use std::io::Read;
use std::net::TcpStream;
use std::sync::Mutex;
use std::{collections::HashSet, io::Write};
use tauri_plugin_store::StoreExt;
mod irc;

type Nick = String;

struct Message(Nick, String);

struct Channel {
    users: HashSet<String>,
    messages: Vec<Message>,
}

struct InternalAppState<'a> {
    clients: irc::Client<'a, TcpStream>,
    channels: HashSet<Channel>,
}

type AppState<'a> = Mutex<InternalAppState<'a>>;

/// Send the message
#[tauri::command]
fn send(state: tauri::State<AppState>) -> String {
    let mut state = state.lock().unwrap();
    let socket = &state.clients.socket;
    "".to_string()
}

/// Run the main program
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // TODO: Move these into the "store"
    let temp_nick = "veryuniquenick";
    let temp_server = "irc.libera.chat:6667";
    // TODO: This has to have error handling
    let tcpstream = TcpStream::connect(temp_server).unwrap();
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(InternalAppState {
            channels: HashSet::new(),
            clients: irc::Client {
                server: temp_server,
                nick: temp_nick,
                real_name: None,
                socket: tcpstream,
                auth: irc::Auth::Plain(temp_nick, None),
            },
        })
        .invoke_handler(tauri::generate_handler![send])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
