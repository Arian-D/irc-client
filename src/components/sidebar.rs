use crate::types::NetworkData;
use leptos::prelude::*;

#[component]
pub fn SidebarLeft(
    #[prop(into)] networks: Signal<Vec<NetworkData>>,
    #[prop(into)] active_network: Signal<String>,
    #[prop(into)] active_channel: Signal<String>,
    set_active_network: WriteSignal<String>,
    set_active_channel: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <aside class="sidebar-left">
            <For
                each=move || networks.get()
                key=|n| n.name.clone()
                children=move |net: NetworkData| {
                    let net_name = net.name.clone();

                    view! {
                        <div class="network-group" style="margin-bottom: 20px;">
                            <h2 style="font-size: 1.1rem; color: #ccc;">{net_name.clone()}</h2>

                            <ul class="channel-list" style="list-style-type: none; padding-left: 10px;">
                                <For
                                    each=move || net.channels.clone()
                                    key=|c| c.clone()
                                    children=move |c: String| {

                                        let click_channel = c.clone();
                                        let click_network = net_name.clone();

                                        let check_channel = c.clone();
                                        let check_network = net_name.clone();

                                        let is_active = move || {
                                            check_channel == active_channel.get() && check_network == active_network.get()
                                        };

                                        view! {
                                            <li
                                                class:active=is_active
                                                on:click=move |_| {
                                                    leptos::logging::log!("CLICKED: Network = {}, Channel = {}", click_network, click_channel);
                                                    set_active_network.set(click_network.clone());
                                                    set_active_channel.set(click_channel.clone());
                                                }
                                                style="cursor: pointer; padding: 5px; margin: 2px 0;"
                                            >
                                                {c}
                                            </li>
                                        }
                                    }
                                />
                            </ul>
                        </div>
                    }
                }
            />
        </aside>
    }
}
