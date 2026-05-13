use log::{debug, info};
use std::net::TcpStream;
use std::sync::Mutex;
use std::{collections::HashSet, io::Write};
use tauri_plugin_store::StoreExt;
mod irc;
use irc::{Command, Message};

type Nick = String;

struct Msg(Nick, String);

struct Channel {
    users: HashSet<String>,
    messages: Vec<Msg>,
}

struct InternalAppState<'a> {
    client: irc::Client<'a, TcpStream>,
    read_buf: Vec<u8>,
    channels: HashSet<Channel>,
}

impl InternalAppState<'_> {
    fn process_messages(&mut self) {
        let messages = self.client.read(&mut self.read_buf);
        for msg in messages {
            debug!("{}", msg);
        }
    }
}

type AppState<'a> = Mutex<InternalAppState<'a>>;

/// Send the message
#[tauri::command]
fn send(state: tauri::State<AppState>) -> String {
    let mut state = state.lock().unwrap();
    let socket = &state.client.socket;
    "".to_string()
}


/// Connect
#[tauri::command]
async fn connect<'a>(state: tauri::State<'_, AppState<'a>>) -> Result<(), ()> {
    let mut state = state.lock().unwrap();
    let mut buf = String::new();
    Ok(())
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
            read_buf: Vec::new(),
            client: irc::Client {
                server: temp_server,
                nick: temp_nick,
                real_name: None,
                socket: tcpstream,
                auth: irc::Auth::Plain(temp_nick, None),
            },
        })
        .invoke_handler(tauri::generate_handler![connect, send])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
