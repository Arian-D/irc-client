use leptos::prelude::*;

#[component]
pub fn MessageBubble(
    name: String,
    content: String,
    // Add is_self to handle layout logic
    #[prop(default = false)] is_self: bool,
    #[prop(default = "blue".to_string())] color: String,
    #[prop(optional)] avatar_url: Option<String>,
) -> impl IntoView {
    let initial = name.chars().next().unwrap_or('?');

    // Determine the alignment class based on who sent the message
    let alignment_class = if is_self { "me" } else { "them" };

    view! {
        <div class=format!("message-wrapper {}", alignment_class)>
            <div class=format!("bubble-container {}", color)>
                <div class="avatar">
                    {move || match &avatar_url {
                        Some(url) => view! { <img src=url.clone() alt="avatar" /> }.into_any(),
                        None => view! { <div class="fallback-avatar">{initial}</div> }.into_any(),
                    }}
                </div>

                <div class="message-content">
                    <span class="sender-name">{name}</span>
                    <p class="text">{content}</p>
                </div>
            </div>
        </div>
    }
}
