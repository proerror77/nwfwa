use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

pub(crate) fn mlops_text_field(label: &'static str, state: &UseStateHandle<String>) -> Html {
    mlops_text_field_with_class(label, state, "")
}

pub(crate) fn mlops_text_field_with_class(
    label: &'static str,
    state: &UseStateHandle<String>,
    extra_class: &'static str,
) -> Html {
    html! {
        <label class={classes!("mlops-field", extra_class)}>
            {label}
            <input
                value={(**state).clone()}
                oninput={{
                    let state = state.clone();
                    Callback::from(move |event: InputEvent| {
                        state.set(event.target_unchecked_into::<HtmlInputElement>().value());
                    })
                }}
            />
        </label>
    }
}

pub(crate) fn mlops_textarea_field(
    label: &'static str,
    state: &UseStateHandle<String>,
    extra_class: &'static str,
) -> Html {
    html! {
        <label class={classes!("mlops-field", extra_class)}>
            {label}
            <textarea
                value={(**state).clone()}
                oninput={{
                    let state = state.clone();
                    Callback::from(move |event: InputEvent| {
                        state.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                    })
                }}
            />
        </label>
    }
}

pub(crate) fn mlops_select_field(
    label: &'static str,
    state: &UseStateHandle<String>,
    options: &'static [&'static str],
) -> Html {
    html! {
        <label class="mlops-field">
            {label}
            <select
                value={(**state).clone()}
                onchange={{
                    let state = state.clone();
                    Callback::from(move |event: Event| {
                        state.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                    })
                }}
            >
                {for options.iter().map(|option| html! {
                    <option value={*option}>{*option}</option>
                })}
            </select>
        </label>
    }
}
