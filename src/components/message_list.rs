use leptos::prelude::*;

use crate::components::message::MessageBubble;
use crate::types::MessageData;

#[component]
pub fn MessageList(history: ReadSignal<Vec<MessageData>>) -> impl IntoView {
    view! {
        <div class="chat-history">
            <For
                each=move || history.get()
                key=|msg: &MessageData| msg.id
                children=|msg: MessageData| {
                    view! {
                        <MessageBubble
                            name=msg.user.clone()
                            content=msg.content.clone()
                            is_self=msg.is_self
                        />
                    }
                }
            />
        </div>
    }
}
