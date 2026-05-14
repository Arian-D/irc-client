use js_sys::Object;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

use crate::components::connection_settings::ConnectionSettingsForm;
use crate::components::input::ChatInput;
use crate::components::message_list::MessageList;
use crate::components::sidebar::SidebarLeft;
use crate::types::{ConnectionSettings, MessageArgs, MessageData};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IrcEvent {
    kind: String,
    channel: Option<String>,
    user: Option<String>,
    text: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateConnectionArgs<'a> {
    settings: &'a ConnectionSettings,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ChannelArgs<'a> {
    channel: &'a str,
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

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        catch,
        js_namespace = ["window", "__TAURI__", "core"],
        js_name = invoke
    )]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen(inline_js = "export function sleep_ms(ms){ return new Promise((resolve) => setTimeout(resolve, ms)); }")]
extern "C" {
    async fn sleep_ms(ms: u32);
}

fn format_invoke_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| {
        js_sys::Reflect::get(&error, &JsValue::from_str("message"))
            .ok()
            .and_then(|msg| msg.as_string())
            .unwrap_or_else(|| "Unknown Tauri invoke error".to_string())
    })
}

async fn invoke_tauri(cmd: &str, args: JsValue) -> Result<JsValue, String> {
    tauri_invoke(cmd, args).await.map_err(format_invoke_error)
}

#[component]
pub fn App() -> impl IntoView {
    let (server_status, set_server_status) = signal(String::new());

    // Mock state messages
    let (history, set_history) = signal(vec![MessageData {
        id: 0,
        channel: "#cachyos".to_string(),
        user: "System".to_string(),
        content: "Welcome to VeryChat!".to_string(),
        is_self: false,
    }]);

    let (next_id, set_next_id) = signal(1);

    let (current_channel, set_current_channel) = signal(String::new());
    let (joined_channels, set_joined_channels) = signal(Vec::<String>::new());

    let (server, set_server) = signal(String::new());
    let (nick, set_nick) = signal(String::new());
    let (real_name, set_real_name) = signal(String::new());
    let (nickserv_account, set_nickserv_account) = signal(String::new());
    let (nickserv_password, set_nickserv_password) = signal(String::new());

    let refresh_joined_channels = Callback::new(move |_| {
        spawn_local(async move {
            let response = match invoke_tauri("get_joined_channels", Object::new().into()).await {
                Ok(response) => response,
                Err(error) => {
                    set_server_status.set(format!("Failed to load joined channels: {error}"));
                    return;
                }
            };
            match serde_wasm_bindgen::from_value::<Vec<String>>(response) {
                Ok(channels) => {
                    let active = current_channel.get_untracked();
                    if channels.is_empty() {
                        set_current_channel.set(String::new());
                    } else if active.is_empty() || !channels.iter().any(|channel| channel == &active)
                    {
                        set_current_channel.set(channels[0].clone());
                    }
                    set_joined_channels.set(channels);
                }
                Err(error) => {
                    set_server_status.set(format!("Failed to load joined channels: {error}"));
                }
            }
        });
    });

    Effect::new(move |_| {
        spawn_local(async move {
            let response = match invoke_tauri("get_connection_settings", Object::new().into()).await
            {
                Ok(response) => response,
                Err(error) => {
                    set_server_status
                        .set(format!("Failed to load connection settings: {error}"));
                    return;
                }
            };
            match serde_wasm_bindgen::from_value::<ConnectionSettings>(response) {
                Ok(settings) => {
                    set_server.set(settings.server);
                    set_nick.set(settings.nick);
                    set_real_name.set(settings.real_name.unwrap_or_default());
                    set_nickserv_account.set(settings.nickserv_account.unwrap_or_default());
                    set_nickserv_password.set(settings.nickserv_password.unwrap_or_default());
                }
                Err(error) => {
                    set_server_status.set(format!("Failed to load connection settings: {error}"));
                }
            }
        });

        refresh_joined_channels.run(());
    });

    Effect::new(move |_| {
        spawn_local(async move {
            loop {
                let response = match invoke_tauri("poll_events", Object::new().into()).await {
                    Ok(response) => response,
                    Err(error) => {
                        set_server_status.set(format!("Failed to poll IRC events: {error}"));
                        sleep_ms(750).await;
                        continue;
                    }
                };

                match serde_wasm_bindgen::from_value::<Vec<IrcEvent>>(response) {
                    Ok(events) => {
                        for event in events {
                            if event.kind == "message" {
                                if let Some(channel) = event.channel {
                                    let current_id = next_id.get_untracked();
                                    set_next_id.update(|id| *id += 1);
                                    let sender = event.user.unwrap_or_else(|| "Unknown".to_string());
                                    let self_nick = nick.get_untracked();
                                    set_history.update(|h| {
                                        h.push(MessageData {
                                            id: current_id,
                                            channel,
                                            user: sender.clone(),
                                            content: event.text.clone(),
                                            is_self: sender == self_nick,
                                        });
                                    });
                                }
                                continue;
                            }

                            if event.kind == "status" {
                                set_server_status.set(event.text);
                                refresh_joined_channels.run(());
                            }
                        }
                    }
                    Err(error) => {
                        set_server_status.set(format!("Failed to decode IRC events: {error}"));
                    }
                }

                sleep_ms(400).await;
            }
        });
    });

    let connect_with_settings = Callback::new(move |_| {
        let settings = ConnectionSettings {
            server: server.get_untracked(),
            nick: nick.get_untracked(),
            real_name: Some(real_name.get_untracked()),
            nickserv_password: Some(nickserv_password.get_untracked()),
            nickserv_account: Some(nickserv_account.get_untracked()),
        };

        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&UpdateConnectionArgs {
                settings: &settings,
            })
            .unwrap();

            let response = match invoke_tauri("update_connection_settings", args).await {
                Ok(response) => response,
                Err(error) => {
                    set_server_status.set(format!("Failed to apply connection settings: {error}"));
                    return;
                }
            };
            match serde_wasm_bindgen::from_value::<String>(response) {
                Ok(message) => {
                    set_server_status.set(message);
                    set_joined_channels.set(Vec::new());
                    set_current_channel.set(String::new());
                    refresh_joined_channels.run(());
                }
                Err(error) => {
                    set_server_status.set(format!("Failed to apply connection settings: {error}"))
                }
            }
        });
    });

    let join_selected_channel = Callback::new(move |channel: String| {
        let Some(channel) = normalize_channel_name(&channel) else {
            set_server_status.set("Channel cannot be empty".to_string());
            return;
        };

        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&ChannelArgs { channel: &channel }).unwrap();
            let response = match invoke_tauri("join_channel", args).await {
                Ok(response) => response,
                Err(error) => {
                    set_server_status.set(format!("Failed to join channel: {error}"));
                    return;
                }
            };
            match serde_wasm_bindgen::from_value::<String>(response) {
                Ok(message) => {
                    set_server_status.set(message.clone());
                    if message.starts_with("Joined ") || message.starts_with("Already joined ") {
                        set_current_channel.set(channel);
                        refresh_joined_channels.run(());
                    }
                }
                Err(error) => {
                    set_server_status.set(format!("Failed to join channel: {error}"));
                }
            }
        });
    });

    let leave_selected_channel = Callback::new(move |channel: String| {
        let Some(channel) = normalize_channel_name(&channel) else {
            set_server_status.set("Channel cannot be empty".to_string());
            return;
        };

        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&ChannelArgs { channel: &channel }).unwrap();
            let response = match invoke_tauri("leave_channel", args).await {
                Ok(response) => response,
                Err(error) => {
                    set_server_status.set(format!("Failed to leave channel: {error}"));
                    return;
                }
            };
            match serde_wasm_bindgen::from_value::<String>(response) {
                Ok(message) => {
                    let success = message.starts_with("Left ");
                    set_server_status.set(message);
                    if success {
                        refresh_joined_channels.run(());
                    }
                }
                Err(error) => {
                    set_server_status.set(format!("Failed to leave channel: {error}"));
                }
            }
        });
    });

    let filtered_history = Signal::derive(move || {
        let active = current_channel.get();
        history
            .get()
            .into_iter()
            .filter(|msg| msg.channel == active)
            .collect::<Vec<_>>()
    });

    let process_message = Callback::new(move |text: String| {
        let channel = current_channel.get_untracked();
        if channel.is_empty() {
            set_server_status.set("Join a channel first".to_string());
            return;
        }
        let sender_nick = nick.get_untracked();
        spawn_local(async move {
            // generate unique id for new message
            let current_id = next_id.get_untracked();
            set_next_id.update(|id| *id += 1);

            // prepare tauri args
            let args = serde_wasm_bindgen::to_value(&MessageArgs {
                channel: &channel,
                message: &text,
            })
            .unwrap();

            let response = match invoke_tauri("send", args).await {
                Ok(response) => response,
                Err(error) => {
                    set_server_status.set(format!("Failed to send message: {error}"));
                    return;
                }
            };
            let response = match serde_wasm_bindgen::from_value::<String>(response) {
                Ok(response) => response,
                Err(error) => {
                    set_server_status.set(format!("Failed to send message: {error}"));
                    return;
                }
            };
            set_server_status.set(response.clone());
            if response != "Message sent" {
                return;
            }

            // update history
            set_history.update(|h| {
                h.push(MessageData {
                    id: current_id,
                    channel: channel.clone(),
                    user: sender_nick.clone(),
                    content: text,
                    is_self: true,
                });
            });
        })
    });

    // TODO: Add channel name to the textbox placeholder

    view! {
        <main class="app-layout">
            <aside class="left-panel">
                <ConnectionSettingsForm
                    server=server
                    set_server=set_server
                    nick=nick
                    set_nick=set_nick
                    real_name=real_name
                    set_real_name=set_real_name
                    nickserv_account=nickserv_account
                    set_nickserv_account=set_nickserv_account
                    nickserv_password=nickserv_password
                    set_nickserv_password=set_nickserv_password
                    on_connect=connect_with_settings
                />
                <SidebarLeft
                    channels=joined_channels
                    active_channel=current_channel
                    set_active_channel=set_current_channel
                    on_join=join_selected_channel
                    on_leave=leave_selected_channel
                />
            </aside>

            <section class="chat-area">
                <header class="chat-header">
                    <h2>"VeryChat" - {move || {
                        let channel = current_channel.get();
                        if channel.is_empty() {
                            "No channel joined".to_string()
                        } else {
                            channel
                        }
                    }}</h2>
                </header>

                <div class="messages-container">
                    <MessageList history=filtered_history />
                </div>

                <div class="input-container">
                    <ChatInput on_send=process_message />
                    <p class="server-status">{move || server_status.get()}</p>
                </div>
            </section>
        </main>
    }
}
