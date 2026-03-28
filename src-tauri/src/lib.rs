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

struct AppState {
    tcp_socket: TcpStream,
    user_nick: Nick,
    channels: HashSet<Channel>,
}

// https://www.retardmaxx.com/
fn user_init(socket: &mut TcpStream, user: &str) -> Result<String, Box<dyn std::error::Error>> {
    socket.write(format!("USER {user} {user} {user} :{user}\n").as_bytes())?;
    socket.write(format!("NICK {user}").as_bytes())?;
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
    let socket = &state.tcp_socket;
    "".to_string()
}

/// Run the main program
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // TLS considered harmful
    // TODO: Maybe have some proper abstraction over IRC instead of rawdogging TCP???
    if let Ok(mut tcpstream) = TcpStream::connect("irc.libera.chat:6667") {
        println!("{tcpstream:#?}");
        // let init = user_init(&mut tcpstream, "veryuniquser");
        // println!("{:#?}", init);

        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .manage(AppState {
                channels: HashSet::new(),
                user_nick: String::new(),
                tcp_socket: tcpstream,

            })
            .invoke_handler(tauri::generate_handler![send])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    } else {
        todo!("Do some error handling.")
    }
}
