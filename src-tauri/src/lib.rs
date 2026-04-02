use std::io::Read;
use std::{collections::HashSet, io::Write};
use std::net::TcpStream;
use log::{info, debug};
mod irc;

type Nick = String;

struct Message(Nick, String);

struct Channel {
    users: HashSet<String>,
    messages: Vec<Message>,
}

struct AppState<'a> {
    client: irc::Client<'a, TcpStream>,
    channels: HashSet<Channel>,
}

// https://www.retardmaxx.com/
fn user_init(socket: &mut TcpStream, user: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut buffer = String::new();
    socket.read_to_string(&mut buffer)?;
    match buffer {
        _ if buffer.is_empty() => Err("🤷".into()),
        bytes => {
            Ok(bytes)
        }
    }
}

/// Send the message
#[tauri::command]
fn send(state: tauri::State<AppState>) -> String {
    let socket = &state.client.socket;
    "".to_string()
}

/// Run the main program
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // T
    let mut tcpstream = TcpStream::connect("irc.libera.chat:6667").unwrap();
    // let init = user_init(&mut tcpstream, "veryuniquser");
    // println!("{:#?}", init);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            channels: HashSet::new(),
            client: irc::Client {
                server: "irc.libera.chat:6667",
                nick: "uniquenick",
                real_name: None,
                socket: tcpstream,
            },

        })
        .invoke_handler(tauri::generate_handler![send])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
