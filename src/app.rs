use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

use crate::components::input::ChatInput;
use crate::components::message_list::MessageList;
use crate::components::sidebar::SidebarLeft;
use crate::types::{MessageArgs, MessageData, NetworkData};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    let (server_status, set_server_status) = signal(String::new());
    const HARDCODED_USER: &str = "Truecel Chud";

    //Mock state messages
    let (history, set_history) = signal(vec![MessageData {
        id: 0,
        user: "System".to_string(),
        content: "Welcome to VeryChat!".to_string(),
        is_self: false,
    }]);

    let (next_id, set_next_id) = signal(1);

    //Mock state channels and users

    let (current_network, set_current_network) = signal("Libera.chat".to_string());
    let (current_channel, set_current_channel) = signal("#cachyos".to_string());

    let (networks, _set_networks) = signal(vec![
        NetworkData {
            name: "Libera.chat".to_string(),
            channels: vec![
                "#rust".to_string(),
                "#cachyos".to_string(),
                "#general".to_string(),
            ],
        },
        NetworkData {
            name: "OFTC".to_string(),
            channels: vec!["#asahi".to_string(), "#linux".to_string()],
        },
    ]);

    let (users, _set_users) = signal(vec![
        "@Truecel Chud".to_string(),
        "+Alice".to_string(),
        "Bob".to_string(),
    ]);

    let process_message = Callback::new(move |text: String| {
        spawn_local(async move {
            //generate unique id for new message
            let current_id = next_id.get_untracked();
            set_next_id.update(|id| *id += 1);

            //prepare tauri args
            let args = serde_wasm_bindgen::to_value(&MessageArgs {
                user: HARDCODED_USER,
                message: &text,
            })
            .unwrap();

            //call backend
            let response = invoke("send", args).await.as_string().unwrap();
            set_server_status.set(response);

            //update history
            set_history.update(|h| {
                h.push(MessageData {
                    id: current_id,
                    user: HARDCODED_USER.to_string(),
                    content: text,
                    is_self: true,
                });
            });
        })
    });

    // TODO: Add channel name to the textbox placeholder

    view! {
        //update css to make 3 columns
        <main class="container">

            <SidebarLeft
                networks=networks
                active_network=current_network
                active_channel=current_channel
                set_active_network=set_current_network
                set_active_channel=set_current_channel
            />

            <h1>"VeryChat"</h1>

            <MessageList history />

            //ToDo: update chatinput to accept current_channel as prop
            <ChatInput on_send=process_message />
            <p style="font-size: 0.8rem; color: gray; margin-top: 10px;">
                { move || server_status.get() }
            </p>
        </main>
    }
}
