use yew::prelude::*;

pub(crate) fn workflow_action_card(
    title: &str,
    description: &str,
    command: &str,
    target: &str,
    tone: &str,
    on_navigate: &Callback<String>,
) -> Html {
    let target = target.to_string();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("workflow-action-card", tone.to_string())}
            onclick={Callback::from(move |_| on_navigate.emit(target.clone()))}
        >
            <span>{title}</span>
            <strong>{command}</strong>
            <small>{description}</small>
        </button>
    }
}
