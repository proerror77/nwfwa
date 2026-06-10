use crate::constants::API_KEY_DEFAULT;
use crate::ops_pages::*;
use crate::ops_routing::{ops_page_from_hash, ops_set_hash, OpsPage, OPS_PAGES};
use crate::state::{ApiKeyContext, Language};
use wasm_bindgen::{closure::Closure, JsCast};
use yew::prelude::*;

#[function_component(OpsApp)]
pub fn ops_app() -> Html {
    let active = use_state(|| {
        ops_page_from_hash(
            &web_sys::window()
                .and_then(|w| w.location().hash().ok())
                .unwrap_or_default(),
        )
    });
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let _language = use_state(|| Language::Zh);

    // Hash routing
    {
        let active = active.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().and_then(|window| {
                let active = active.clone();
                let callback =
                    Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                        let hash = web_sys::window()
                            .and_then(|w| w.location().hash().ok())
                            .unwrap_or_default();
                        active.set(ops_page_from_hash(&hash));
                    }));
                window
                    .add_event_listener_with_callback(
                        "hashchange",
                        callback.as_ref().unchecked_ref(),
                    )
                    .ok()?;
                Some((window, callback))
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

    html! {
        <ContextProvider<ApiKeyContext> context={ApiKeyContext(api_key.clone())}>
        <div class="app">
            // ── Sidebar (same visual language as original platform) ──────────
            <aside class="sidebar">
                <div class="brand-block">
                    <span>{"FWA PLATFORM"}</span>
                    <h1>{"风控运营"}</h1>
                    <p>{"保险理赔 FWA 检测与审核"}</p>
                </div>

                // API key input
                <div style="padding: 8px 10px 0;">
                    <label style="font-size:11px;color:rgba(226,239,255,0.5);margin:0 0 4px;display:block;">
                        {"API Key"}
                    </label>
                    <input
                        type="text"
                        style="background:rgba(255,255,255,0.07);border:1px solid rgba(226,239,255,0.15);color:#f0f7ff;border-radius:6px;padding:6px 8px;font-size:11px;width:100%;"
                        placeholder="输入 API Key"
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

                <nav class="module-nav" aria-label="FWA operations modules" style="margin-top:16px;">
                    {for OPS_PAGES.iter().map(|&page| {
                        let navigate = navigate.clone();
                        let is_active = *active == page;
                        html! {
                            <button
                                class={classes!(is_active.then_some("active"))}
                                onclick={Callback::from(move |_| navigate.emit(page))}
                            >
                                <span class={classes!("nav-icon", page.icon_class())}></span>
                                <span class="nav-copy">
                                    <span class="nav-label">{page.label()}</span>
                                    <span class="nav-description">{page.description()}</span>
                                </span>
                            </button>
                        }
                    })}
                </nav>

                <div style="padding:16px 10px 8px;border-top:1px solid rgba(226,239,255,0.1);margin-top:auto;">
                    <span style="font-size:10px;color:rgba(226,239,255,0.35);display:flex;align-items:center;gap:6px;">
                        <span style="width:6px;height:6px;border-radius:50%;background:#34d399;display:inline-block;box-shadow:0 0 0 2px rgba(52,211,153,0.3);"></span>
                        {"实时运营"}
                    </span>
                </div>
            </aside>

            // ── Main workspace ───────────────────────────────────────────────
            <main class="workspace">
                <div class="workspace-topbar">
                    <div class="topbar-context">
                        <span class="eyebrow">{"FWA 风控平台"}</span>
                        <strong>{page_context(*active)}</strong>
                    </div>
                    <div class="topbar-actions">
                        <span class="api-chip status-live">{"运营中"}</span>
                    </div>
                </div>
                <div class="workspace-content">
                    {match *active {
                        OpsPage::ClaimsQueue     => html! { <ClaimsQueuePage /> },
                        OpsPage::ReviewWorkbench => html! { <ReviewWorkbenchPage /> },
                        OpsPage::CaseTracker     => html! { <CaseTrackerPage /> },
                        OpsPage::RuleLibrary     => html! { <RuleLibraryPage /> },
                        OpsPage::Dashboard       => html! { <OpsDashboardPage /> },
                    }}
                </div>
            </main>
        </div>
        </ContextProvider<ApiKeyContext>>
    }
}

fn page_context(page: OpsPage) -> &'static str {
    match page {
        OpsPage::ClaimsQueue     => "今日理赔队列",
        OpsPage::ReviewWorkbench => "审核工作台",
        OpsPage::CaseTracker     => "案件追踪",
        OpsPage::RuleLibrary     => "规则库",
        OpsPage::Dashboard       => "运营仪表盘",
    }
}
