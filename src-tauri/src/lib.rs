use log::error;
use std::net::TcpStream;
use std::sync::Mutex;
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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendArgs {
    channel: String,
    message: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChannelArgs {
    channel: String,
}

/// Send the message
#[tauri::command]
fn send(args: SendArgs, state: tauri::State<'_, AppState>) -> String {
    let channel = args.channel.trim();
    let message = args.message.trim();
    if channel.is_empty() {
        return "Channel cannot be empty".to_string();
    }
    if message.is_empty() {
        return "Message cannot be empty".to_string();
    }

    let mut client = match state.client.lock() {
        Ok(client) => client,
        Err(error) => return format!("Failed to access IRC client: {error}"),
    };
    let joined_channels = match state.joined_channels.lock() {
        Ok(channels) => channels,
        Err(error) => return format!("Failed to access joined channels: {error}"),
    };

    if !joined_channels.contains(channel) {
        return format!("Not joined to {channel}. Join it first.");
    }

    if let Err(error) = client.send_privmsg(channel, message) {
        return format!("Failed to send message: {error}");
    }

    "Message sent".to_string()
}

#[tauri::command]
fn join_channel(args: ChannelArgs, state: tauri::State<'_, AppState>) -> String {
    let channel = args.channel.trim();
    if channel.is_empty() {
        return "Channel cannot be empty".to_string();
    }

    let mut client = match state.client.lock() {
        Ok(client) => client,
        Err(error) => return format!("Failed to access IRC client: {error}"),
    };
    let mut joined_channels = match state.joined_channels.lock() {
        Ok(channels) => channels,
        Err(error) => return format!("Failed to access joined channels: {error}"),
    };

    if joined_channels.contains(channel) {
        return format!("Already joined {channel}");
    }

    if let Err(error) = client.join_channel(channel) {
        return format!("Failed to join {channel}: {error}");
    }
    joined_channels.insert(channel.to_string());
    format!("Joined {channel}")
}

#[tauri::command]
fn leave_channel(args: ChannelArgs, state: tauri::State<'_, AppState>) -> String {
    let channel = args.channel.trim();
    if channel.is_empty() {
        return "Channel cannot be empty".to_string();
    }

    let mut client = match state.client.lock() {
        Ok(client) => client,
        Err(error) => return format!("Failed to access IRC client: {error}"),
    };
    let mut joined_channels = match state.joined_channels.lock() {
        Ok(channels) => channels,
        Err(error) => return format!("Failed to access joined channels: {error}"),
    };

    if !joined_channels.contains(channel) {
        return format!("Not currently joined to {channel}");
    }

    if let Err(error) = client.leave_channel(channel) {
        return format!("Failed to leave {channel}: {error}");
    }
    joined_channels.remove(channel);
    format!("Left {channel}")
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
            get_joined_channels,
            get_connection_settings,
            update_connection_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
