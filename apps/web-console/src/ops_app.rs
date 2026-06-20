use crate::constants::API_KEY_DEFAULT;
use crate::i18n::tr;
use crate::ops_pages::*;
use crate::ops_routing::{
    ops_page_from_hash, ops_set_hash, ops_set_hash_with_id, ops_sub_id_from_hash, OpsPage,
    OPS_NAV_GROUPS,
};
use crate::pages::*;
use crate::state::{ApiKeyContext, Language};
use wasm_bindgen::{closure::Closure, JsCast};
use yew::prelude::*;

pub(crate) fn ops_entry_card(
    page: OpsPage,
    eyebrow: &'static str,
    title: &'static str,
    body: &'static str,
    tone: &'static str,
    navigate: &Callback<OpsPage>,
) -> Html {
    let navigate = navigate.clone();
    html! {
        <button
            class={classes!("workflow-action-card", tone)}
            onclick={Callback::from(move |_: MouseEvent| navigate.emit(page))}
        >
            <span>{eyebrow}</span>
            <strong>{title}</strong>
            <small>{body}</small>
        </button>
    }
}

#[function_component(OpsApp)]
pub fn ops_app() -> Html {
    let initial_hash = web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default();
    let active = use_state(|| ops_page_from_hash(&initial_hash));
    // Deep-link sub-id: `#investigate?id=CASE-001` pre-selects a case in the workbench.
    let deep_link_id: UseStateHandle<Option<String>> =
        use_state(|| ops_sub_id_from_hash(&initial_hash));
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let language = use_state(|| Language::Zh);

    // Set <html lang> on language change.
    {
        let language = *language;
        use_effect_with(language, move |language| {
            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                if let Some(root) = document.document_element() {
                    let _ = root.set_attribute("lang", language.document_code());
                }
            }
            || ()
        });
    }

    // Hash routing.
    {
        let active = active.clone();
        let deep_link_id = deep_link_id.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().and_then(|window| {
                let active = active.clone();
                let deep_link_id = deep_link_id.clone();
                let cb = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                    let hash = web_sys::window()
                        .and_then(|w| w.location().hash().ok())
                        .unwrap_or_default();
                    active.set(ops_page_from_hash(&hash));
                    deep_link_id.set(ops_sub_id_from_hash(&hash));
                }));
                window
                    .add_event_listener_with_callback("hashchange", cb.as_ref().unchecked_ref())
                    .ok()?;
                Some((window, cb))
            });
            move || {
                if let Some((window, cb)) = listener {
                    let _ = window.remove_event_listener_with_callback(
                        "hashchange",
                        cb.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    let navigate = {
        let active = active.clone();
        Callback::from(move |page: OpsPage| {
            ops_set_hash(page);
            active.set(page);
        })
    };

    let toggle_language = {
        let language = language.clone();
        Callback::from(move |_: MouseEvent| language.set((*language).toggle()))
    };

    // Navigate to ActionQueue (called from Dashboard action counters).
    let go_to_queue = {
        let navigate = navigate.clone();
        Callback::from(move |_: MouseEvent| navigate.emit(OpsPage::ActionQueue))
    };

    // Open Investigation workbench for a specific case / lead id.
    let open_case = {
        let active = active.clone();
        let deep_link_id = deep_link_id.clone();
        Callback::from(move |case_id: String| {
            ops_set_hash_with_id(OpsPage::Investigate, &case_id);
            deep_link_id.set(Some(case_id));
            active.set(OpsPage::Investigate);
        })
    };

    // After submitting an investigation conclusion, return to ActionQueue.
    let on_investigation_done = {
        let navigate = navigate.clone();
        let deep_link_id = deep_link_id.clone();
        Callback::from(move |_: ()| {
            deep_link_id.set(None);
            navigate.emit(OpsPage::ActionQueue);
        })
    };

    let language = *language;

    html! {
        <ContextProvider<ApiKeyContext> context={ApiKeyContext(api_key.clone())}>
        <div class="app">

            // ── Sidebar ──────────────────────────────────────────────────────
            <aside class="sidebar">
                <div class="brand-block">
                    <span>{"FWA PLATFORM"}</span>
                    <h1>{tr(language, "FWA Operations", "风控运营")}</h1>
                    <p>{tr(language, "Claims FWA Operations Console", "保险理赔 FWA 运营控制台")}</p>
                </div>

                // API key (pilot)
                <div style="padding:6px 10px 12px;">
                    <input
                        type="text"
                        style="background:rgba(255,255,255,0.07);border:1px solid rgba(226,239,255,0.15);color:#f0f7ff;border-radius:6px;padding:5px 8px;font-size:11px;width:100%;box-sizing:border-box;"
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

                // Navigation — 4 pages only
                <nav class="module-nav" aria-label="FWA operations">
                    { for OPS_NAV_GROUPS.iter().flat_map(|g| g.pages()).map(|&page| {
                        let navigate = navigate.clone();
                        let is_active = *active == page;
                        html! {
                            <button
                                class={classes!(is_active.then_some("active"))}
                                onclick={Callback::from(move |_| navigate.emit(page))}
                            >
                                <span class={classes!("nav-icon", page.icon_class())}></span>
                                <span class="nav-copy">
                                    <span class="nav-label">
                                        <span>{page.label_for(language)}</span>
                                    </span>
                                    <span class="nav-description">
                                        <span>{page.description_for(language)}</span>
                                    </span>
                                </span>
                            </button>
                        }
                    }) }
                </nav>

                <div style="padding:12px 10px 8px;border-top:1px solid rgba(226,239,255,0.1);margin-top:8px;">
                    <span style="font-size:10px;color:rgba(226,239,255,0.35);display:flex;align-items:center;gap:6px;">
                        <span style="width:6px;height:6px;border-radius:50%;background:#34d399;flex-shrink:0;box-shadow:0 0 0 2px rgba(52,211,153,0.3);display:inline-block;"></span>
                        {tr(language, "Live operations", "实时运营")}
                    </span>
                </div>
            </aside>

            // ── Workspace ────────────────────────────────────────────────────
            <main class="workspace">
                <div class="workspace-topbar">
                    <div class="topbar-context">
                        <span class="eyebrow">{tr(language, "FWA Platform", "FWA 风控平台")}</span>
                        <strong>{(*active).label_for(language)}</strong>
                        <small>{(*active).description_for(language)}</small>
                    </div>
                    <div class="topbar-actions">
                        <button class="language-toggle" onclick={toggle_language}>
                            {match language {
                                Language::Zh => "English",
                                Language::En => "中文",
                            }}
                        </button>
                        <span class="api-chip status-live">{tr(language, "Live", "运营中")}</span>
                    </div>
                </div>

                <div class="workspace-content">
                    { match *active {
                        OpsPage::Dashboard => html! {
                            <OpsDashboardPage
                                language={language}
                                on_go_to_queue={go_to_queue.clone()}
                            />
                        },
                        OpsPage::ActionQueue => html! {
                            <ActionQueuePage
                                language={language}
                                on_open_case={open_case.clone()}
                            />
                        },
                        OpsPage::Investigate => html! {
                            <InvestigateWorkbenchPage
                                language={language}
                                initial_case_id={(*deep_link_id).clone()}
                                on_done={on_investigation_done.clone()}
                            />
                        },
                        OpsPage::SystemLearning => html! {
                            <SystemLearningPage language={language} />
                        },
                    } }
                </div>
            </main>

        </div>
        </ContextProvider<ApiKeyContext>>
    }
}
