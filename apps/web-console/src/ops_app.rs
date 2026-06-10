use crate::ops_routing::{ops_page_from_hash, ops_set_hash, OpsPage, OPS_PAGES};
use crate::ops_pages::*;
use crate::state::{use_api_key, ApiKeyContext, Language};
use crate::constants::API_KEY_DEFAULT;
use wasm_bindgen::{closure::Closure, JsCast};
use yew::prelude::*;

#[function_component(OpsApp)]
pub fn ops_app() -> Html {
    let active = use_state(|| ops_page_from_hash(
        &web_sys::window()
            .and_then(|w| w.location().hash().ok())
            .unwrap_or_default()
    ));
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let language = use_state(|| Language::Zh); // default Chinese for insurance ops

    // Hash routing
    {
        let active = active.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().and_then(|window| {
                let active = active.clone();
                let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                    let hash = web_sys::window()
                        .and_then(|w| w.location().hash().ok())
                        .unwrap_or_default();
                    active.set(ops_page_from_hash(&hash));
                }));
                window.add_event_listener_with_callback("hashchange", callback.as_ref().unchecked_ref()).ok()?;
                Some((window, callback))
            });
            move || { if let Some((window, cb)) = listener {
                let _ = window.remove_event_listener_with_callback("hashchange", cb.as_ref().unchecked_ref());
            }}
        });
    }

    let navigate = {
        let active = active.clone();
        Callback::from(move |page: OpsPage| {
            ops_set_hash(page);
            active.set(page);
        })
    };

    html! {
        <ContextProvider<ApiKeyContext> context={ApiKeyContext(api_key.clone())}>
        <div class="ops-layout">
            // Topbar
            <header class="ops-topbar">
                <div class="ops-topbar-brand">
                    <strong>{"FWA 风控平台"}</strong>
                    <span>{"Insurance Operations"}</span>
                </div>
                <nav class="ops-topbar-nav">
                    {for OPS_PAGES.iter().map(|&page| {
                        let navigate = navigate.clone();
                        let is_active = *active == page;
                        html! {
                            <button
                                class={classes!(is_active.then_some("ops-nav-active"))}
                                onclick={Callback::from(move |_| navigate.emit(page))}
                            >
                                {page.label()}
                            </button>
                        }
                    })}
                </nav>
                <div class="ops-topbar-right">
                    <span class="ops-live-dot"></span>
                    <span>{"实时运营"}</span>
                    // API key input for pilot
                    <input
                        type="text"
                        style="background:rgba(255,255,255,0.08);border:1px solid rgba(255,255,255,0.15);color:#f0f7ff;border-radius:5px;padding:3px 8px;font-size:11px;width:160px;"
                        placeholder="API Key"
                        value={(*api_key).clone()}
                        oninput={{
                            let api_key = api_key.clone();
                            Callback::from(move |e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                api_key.set(input.value());
                            })
                        }}
                    />
                </div>
            </header>
            // Main workspace
            <main class="ops-workspace">
                {match *active {
                    OpsPage::ClaimsQueue     => html! { <ClaimsQueuePage /> },
                    OpsPage::ReviewWorkbench => html! { <ReviewWorkbenchPage /> },
                    OpsPage::CaseTracker     => html! { <CaseTrackerPage /> },
                    OpsPage::RuleLibrary     => html! { <RuleLibraryPage /> },
                    OpsPage::Dashboard       => html! { <OpsDashboardPage /> },
                }}
            </main>
        </div>
        </ContextProvider<ApiKeyContext>>
    }
}
