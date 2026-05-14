use log::error;
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;
use std::{collections::HashSet, env};
mod irc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionSettings {
    server: String,
    nick: String,
    real_name: Option<String>,
    nickserv_password: Option<String>,
    nickserv_account: Option<String>,
}

struct AppState {
    client: Mutex<irc::Client<'static, TcpStream>>,
    settings: Mutex<ConnectionSettings>,
    joined_channels: Mutex<HashSet<String>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IrcEvent {
    kind: String,
    channel: Option<String>,
    user: Option<String>,
    text: String,
}

fn normalize_channel_name(input: &str) -> Option<String> {
    let channel = input.trim().trim_start_matches(':').trim();
    if channel.is_empty() {
        return None;
    }
    if channel.starts_with('#') {
        Some(channel.to_string())
    } else {
        Some(format!("#{channel}"))
    }
}

fn source_nick(source: Option<&str>) -> Option<String> {
    source.map(|raw| raw.split('!').next().unwrap_or(raw).to_string())
}

fn combine_params(params: &[&str], start_index: usize) -> String {
    if params.len() <= start_index {
        return String::new();
    }
    params[start_index..]
        .join(" ")
        .trim_start_matches(':')
        .to_string()
}

/// Send the message
#[tauri::command]
fn send(channel: String, message: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let Some(channel) = normalize_channel_name(&channel) else {
        return Err("Channel cannot be empty".to_string());
    };
    let message = message.trim();
    if message.is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    let mut client = state
        .client
        .lock()
        .map_err(|error| format!("Failed to access IRC client: {error}"))?;
    client
        .service_connection()
        .map_err(|error| format!("Connection error before send: {error}"))?;
    let joined_channels = state
        .joined_channels
        .lock()
        .map_err(|error| format!("Failed to access joined channels: {error}"))?;

    if !joined_channels.contains(channel.as_str()) {
        return Err(format!("Not joined to {channel}. Join it first."));
    }

    client
        .send_privmsg(channel.as_str(), message)
        .map_err(|error| format!("Failed to send message: {error}"))?;

    Ok("Message sent".to_string())
}

#[tauri::command]
fn join_channel(channel: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let Some(channel) = normalize_channel_name(&channel) else {
        return Err("Channel cannot be empty".to_string());
    };

    let mut client = state
        .client
        .lock()
        .map_err(|error| format!("Failed to access IRC client: {error}"))?;
    client
        .service_connection()
        .map_err(|error| format!("Connection error before join: {error}"))?;
    let mut joined_channels = state
        .joined_channels
        .lock()
        .map_err(|error| format!("Failed to access joined channels: {error}"))?;

    if joined_channels.contains(channel.as_str()) {
        return Ok(format!("Already joined {channel}"));
    }

    client
        .join_channel(channel.as_str())
        .map_err(|error| format!("Failed to join {channel}: {error}"))?;
    joined_channels.insert(channel.clone());
    Ok(format!("Joined {channel}"))
}

#[tauri::command]
fn leave_channel(channel: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let Some(channel) = normalize_channel_name(&channel) else {
        return Err("Channel cannot be empty".to_string());
    };

    let mut client = state
        .client
        .lock()
        .map_err(|error| format!("Failed to access IRC client: {error}"))?;
    client
        .service_connection()
        .map_err(|error| format!("Connection error before leave: {error}"))?;
    let mut joined_channels = state
        .joined_channels
        .lock()
        .map_err(|error| format!("Failed to access joined channels: {error}"))?;

    if !joined_channels.contains(channel.as_str()) {
        return Err(format!("Not currently joined to {channel}"));
    }

    client
        .leave_channel(channel.as_str())
        .map_err(|error| format!("Failed to leave {channel}: {error}"))?;
    joined_channels.remove(channel.as_str());
    Ok(format!("Left {channel}"))
}

#[tauri::command]
fn get_joined_channels(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut channels = state
        .joined_channels
        .lock()
        .map_err(|error| format!("Failed to access joined channels: {error}"))?
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    channels.sort();
    Ok(channels)
}

#[tauri::command]
fn poll_events(state: tauri::State<'_, AppState>) -> Result<Vec<IrcEvent>, String> {
    let mut client = state
        .client
        .lock()
        .map_err(|error| format!("Failed to access IRC client: {error}"))?;
    let mut joined_channels = state
        .joined_channels
        .lock()
        .map_err(|error| format!("Failed to access joined channels: {error}"))?;

    let mut buf = Vec::new();
    let messages = client.read(&mut buf);
    let mut events = Vec::new();

    for message in messages {
        if message.command == "PING" {
            if let Some(token) = message.params.as_ref().and_then(|params| params.last().copied()) {
                client
                    .send_pong(token)
                    .map_err(|error| format!("Failed to send PONG: {error}"))?;
            }
            continue;
        }

        match message.command {
            "PRIVMSG" => {
                if let Some(params) = message.params.as_ref() {
                    if let Some(channel) = params.first() {
                        events.push(IrcEvent {
                            kind: "message".to_string(),
                            channel: Some((*channel).to_string()),
                            user: source_nick(message.source),
                            text: combine_params(params, 1),
                        });
                    }
                }
            }
            "JOIN" => {
                if let Some(params) = message.params.as_ref() {
                    if let Some(channel) = params.first() {
                        let normalized = normalize_channel_name(channel).unwrap_or_else(|| channel.to_string());
                        if source_nick(message.source).as_deref() == Some(client.nick) {
                            joined_channels.insert(normalized.clone());
                        }
                        events.push(IrcEvent {
                            kind: "status".to_string(),
                            channel: Some(normalized),
                            user: source_nick(message.source),
                            text: "Joined".to_string(),
                        });
                    }
                }
            }
            "PART" => {
                if let Some(params) = message.params.as_ref() {
                    if let Some(channel) = params.first() {
                        let normalized = normalize_channel_name(channel).unwrap_or_else(|| channel.to_string());
                        if source_nick(message.source).as_deref() == Some(client.nick) {
                            joined_channels.remove(normalized.as_str());
                        }
                        events.push(IrcEvent {
                            kind: "status".to_string(),
                            channel: Some(normalized),
                            user: source_nick(message.source),
                            text: "Left".to_string(),
                        });
                    }
                }
            }
            "ERROR" | "NOTICE" | "401" | "403" | "404" | "433" => {
                let text = message
                    .params
                    .as_ref()
                    .map(|params| combine_params(params, 0))
                    .unwrap_or_else(|| "IRC server message".to_string());
                events.push(IrcEvent {
                    kind: "status".to_string(),
                    channel: None,
                    user: source_nick(message.source),
                    text,
                });
            }
            _ => {}
        }
    }

    Ok(events)
}

#[tauri::command]
fn get_connection_settings(
    state: tauri::State<'_, AppState>,
) -> Result<ConnectionSettings, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|error| format!("Failed to read settings: {error}"))
}

#[tauri::command]
fn update_connection_settings(
    settings: ConnectionSettings,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let normalized = normalize_settings(settings);
    let client = connect_client_from_settings(&normalized)?;

    {
        let mut client_guard = state
            .client
            .lock()
            .map_err(|error| format!("Failed to update IRC client: {error}"))?;
        *client_guard = client;
    }

    {
        let mut settings_guard = state
            .settings
            .lock()
            .map_err(|error| format!("Failed to persist settings in app state: {error}"))?;
        *settings_guard = normalized;
    }
    {
        let mut joined_channels = state
            .joined_channels
            .lock()
            .map_err(|error| format!("Failed to reset joined channel list: {error}"))?;
        joined_channels.clear();
    }

    Ok("Connected with updated settings".to_string())
}

fn normalize_optional_field(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_settings(settings: ConnectionSettings) -> ConnectionSettings {
    ConnectionSettings {
        server: settings.server.trim().to_string(),
        nick: settings.nick.trim().to_string(),
        real_name: normalize_optional_field(settings.real_name),
        nickserv_password: normalize_optional_field(settings.nickserv_password),
        nickserv_account: normalize_optional_field(settings.nickserv_account),
    }
}

fn leak_string(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn connect_client_from_settings(
    settings: &ConnectionSettings,
) -> Result<irc::Client<'static, TcpStream>, String> {
    if settings.server.is_empty() {
        return Err("Server cannot be empty".to_string());
    }
    if settings.nick.is_empty() {
        return Err("Nick cannot be empty".to_string());
    }

    let tcpstream = TcpStream::connect(&settings.server)
        .map_err(|error| format!("Failed to connect to {}: {error}", settings.server))?;
    tcpstream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|error| format!("Failed to configure IRC socket read timeout: {error}"))?;

    let auth = if let Some(password) = settings.nickserv_password.clone() {
        irc::Auth::NickServ {
            account: settings.nickserv_account.clone().map(leak_string),
            password: leak_string(password),
        }
    } else {
        irc::Auth::None
    };

    let mut client = irc::Client {
        server: leak_string(settings.server.clone()),
        nick: leak_string(settings.nick.clone()),
        real_name: settings.real_name.clone().map(leak_string),
        socket: tcpstream,
        auth,
    };

    client
        .register_and_authenticate()
        .map_err(|error| format!("Failed during IRC registration/authentication: {error}"))?;
    client
        .await_welcome(Duration::from_secs(8))
        .map_err(|error| format!("Failed waiting for IRC server welcome: {error}"))?;

    Ok(client)
}

/// Run the main program
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_settings = normalize_settings(ConnectionSettings {
        server: env::var("IRC_SERVER").unwrap_or_else(|_| "irc.libera.chat:6667".to_string()),
        nick: env::var("IRC_NICK").unwrap_or_else(|_| "uniquenick".to_string()),
        real_name: env::var("IRC_REAL_NAME").ok(),
        nickserv_password: env::var("IRC_NICKSERV_PASSWORD").ok(),
        nickserv_account: env::var("IRC_NICKSERV_ACCOUNT").ok(),
    });

    let client = connect_client_from_settings(&initial_settings).unwrap_or_else(|error| {
        error!("Initial IRC setup failed: {error}");
        panic!("Initial IRC setup failed: {error}");
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            client: Mutex::new(client),
            settings: Mutex::new(initial_settings),
            joined_channels: Mutex::new(HashSet::new()),
        })
        .invoke_handler(tauri::generate_handler![
            send,
            join_channel,
            leave_channel,
            poll_events,
            get_joined_channels,
            get_connection_settings,
            update_connection_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
