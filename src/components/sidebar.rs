use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::prelude::*;

#[component]
pub fn SidebarLeft(
    #[prop(into)] channels: Signal<Vec<String>>,
    #[prop(into)] active_channel: Signal<String>,
    set_active_channel: WriteSignal<String>,
    #[prop(into)] on_join: Callback<String>,
    #[prop(into)] on_leave: Callback<String>,
) -> impl IntoView {
    let (channel_draft, set_channel_draft) = signal(String::new());

    let join_channel = move |ev: SubmitEvent| {
        ev.prevent_default();
        let channel = channel_draft.get_untracked();
        if channel.trim().is_empty() {
            return;
        }
        on_join.run(channel);
        set_channel_draft.set(String::new());
    };

    view! {
        <aside class="sidebar-left">
            <div class="network-group">
                <h2>"Libera.chat"</h2>

                <form class="join-channel-form" on:submit=join_channel>
                    <input
                        placeholder="#channel"
                        prop:value=move || channel_draft.get()
                        on:input=move |ev| set_channel_draft.set(event_target_value(&ev))
                    />
                    <button type="submit">"Join"</button>
                </form>

                <ul class="channel-list">
                    <For
                        each=move || channels.get()
                        key=|channel| channel.clone()
                        children=move |channel: String| {
                            let select_channel = channel.clone();
                            let leave_channel = channel.clone();
                            let check_channel = channel.clone();
                            let is_active = move || check_channel == active_channel.get();

                            view! {
                                <li class:active=is_active on:click=move |_| set_active_channel.set(select_channel.clone())>
                                    <span>{channel}</span>
                                    <button
                                        class="leave-channel-btn"
                                        type="button"
                                        on:click=move |ev: MouseEvent| {
                                            ev.stop_propagation();
                                            on_leave.run(leave_channel.clone());
                                        }
                                    >
                                        "Leave"
                                    </button>
                                </li>
                            }
                        }
                    />
                </ul>
            </div>
        </aside>
    }
}
