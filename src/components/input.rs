use leptos::ev::SubmitEvent;
use leptos::prelude::*;

#[component]
pub fn ChatInput(#[prop(into)] on_send: Callback<String>) -> impl IntoView {
    let (draft, set_draft) = signal(String::new());

    let update_draft = move |ev| {
        let v = event_target_value(&ev);
        set_draft.set(v);
    };

    let submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let text = draft.get_untracked();
        if text.is_empty() {
            return;
        }

        set_draft.set(String::new()); //clear the box

        //HAND THE TEXT TO THE CALLBACK
        on_send.run(text);
    };

    view! {
        <form class="row" on:submit=submit>
            <input
                id="greet-input"
                placeholder="Message"
                prop:value=move || draft.get()
                on:input=update_draft
            />
            <button type="submit">"▶"</button>
        </form>
    }
}
