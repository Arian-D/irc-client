use leptos::ev::SubmitEvent;
use leptos::prelude::*;

#[component]
pub fn ConnectionSettingsForm(
    #[prop(into)] server: Signal<String>,
    set_server: WriteSignal<String>,
    #[prop(into)] nick: Signal<String>,
    set_nick: WriteSignal<String>,
    #[prop(into)] real_name: Signal<String>,
    set_real_name: WriteSignal<String>,
    #[prop(into)] nickserv_account: Signal<String>,
    set_nickserv_account: WriteSignal<String>,
    #[prop(into)] nickserv_password: Signal<String>,
    set_nickserv_password: WriteSignal<String>,
    #[prop(into)] on_connect: Callback<()>,
) -> impl IntoView {
    let submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        on_connect.run(());
    };

    view! {
        <form class="connection-settings" on:submit=submit>
            <h3>"Connection"</h3>
            <input
                placeholder="Server (host:port)"
                prop:value=move || server.get()
                on:input=move |ev| set_server.set(event_target_value(&ev))
            />
            <input
                placeholder="Nick"
                prop:value=move || nick.get()
                on:input=move |ev| set_nick.set(event_target_value(&ev))
            />
            <input
                placeholder="Real name (optional)"
                prop:value=move || real_name.get()
                on:input=move |ev| set_real_name.set(event_target_value(&ev))
            />
            <input
                placeholder="NickServ account (optional)"
                prop:value=move || nickserv_account.get()
                on:input=move |ev| set_nickserv_account.set(event_target_value(&ev))
            />
            <input
                type="password"
                placeholder="NickServ password (optional)"
                prop:value=move || nickserv_password.get()
                on:input=move |ev| set_nickserv_password.set(event_target_value(&ev))
            />
            <button type="submit">"Connect"</button>
        </form>
    }
}
