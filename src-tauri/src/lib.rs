use log::{debug, info};
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;
use std::collections::HashSet;
use tauri::Manager;
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

type AppState = Mutex<InternalAppState<'static>>;

fn lock_state<'a>(state: &'a tauri::State<'_, AppState>) -> std::sync::MutexGuard<'a, InternalAppState<'static>> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

/// Send the message
#[tauri::command]
fn send(state: tauri::State<AppState>, target: String, text: String) {
    let mut state = lock_state(&state);
    state.client.send(Command::PrivMsg {
        message: text,
        receivers: vec![target],
    });
}


/// Connect: initiate IRC connection by sending NICK then USER
#[tauri::command]
async fn connect(state: tauri::State<'_, AppState>) -> Result<(), ()> {
    let mut state = lock_state(&state);
    let nick = state.client.nick.to_string();
    let real_name = state.client.real_name.unwrap_or(state.client.nick).to_string();
    state.client.send(Command::Nick { nickname: nick.clone() });
    state.client.send(Command::User { username: nick, real_name });
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
    tcpstream.set_nonblocking(true).unwrap();
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(InternalAppState {
            channels: HashSet::new(),
            read_buf: Vec::new(),
            client: irc::Client {
                server: temp_server,
                nick: temp_nick,
                real_name: None,
                socket: tcpstream,
                auth: irc::Auth::Plain(temp_nick, None),
            },
        }))
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    let state = handle.state::<AppState>();
                    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                    state.process_messages();
                    std::thread::sleep(Duration::from_millis(10));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![connect, send])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
