use leptos::attr::NextAttribute;
use leptos::math::Mover;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::components::input::ChatInput;
use crate::components::message_list::MessageList;
use crate::types::{MessageArgs, MessageData};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    let (server_status, set_server_status) = signal(String::new());
    const HARDCODED_USER: &str = "Truecel Chud";

    let (history, set_history) = signal(vec![MessageData {
        id: 0,
        user: "System".to_string(),
        content: "Welcome to VeryChat!".to_string(),
        is_self: false,
    }]);

    let (next_id, set_next_id) = signal(1);

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
        <main class="container">
            <h1>"VeryChat"</h1>

            <MessageList history />

            <ChatInput on_send=process_message />
            <p style="font-size: 0.8rem; color: gray; margin-top: 10px;">
                { move || server_status.get() }
            </p>
        </main>
    }
}
