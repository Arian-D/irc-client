use leptos::task::spawn_local;
use leptos::{ev::SubmitEvent, prelude::*};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct MessageArgs<'a> {
    user: &'a str,
    message: &'a str,
}

#[component]
pub fn App() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (msg, set_msg) = signal(String::new());
    const HARDCODED_USER: &str = "Luke Smith";

    let update_name = move |ev| {
        let v = event_target_value(&ev);
        set_name.set(v);
    };

    let send = move |ev: SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let name = name.get_untracked();
            if name.is_empty() {
                return;
            }

            let args = serde_wasm_bindgen::to_value(&MessageArgs { user: HARDCODED_USER, message: &name }).unwrap();
            // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
            let new_msg = invoke("send", args).await.as_string().unwrap();
            set_msg.set(new_msg);
        });
    };
    // TODO: Add channel name to the textbox placeholder
    view! {
        <main class="container">
            <h1>"VeryChat"</h1>
            <form class="row" on:submit=send>
                <input
                    id="greet-input"
                    placeholder="Message"
                    on:input=update_name
                />
                <button type="submit">"▶"</button>
            </form>
            <p>{ move || msg.get() }</p>
        </main>
    }
}
