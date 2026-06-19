use crate::constants::API_KEY_DEFAULT;
use crate::i18n::tr;
use crate::ops_pages::*;
use crate::ops_routing::{
    ops_page_from_hash, ops_set_hash, ops_sub_id_from_hash, OpsPage, OPS_NAV_GROUPS,
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
    /// Deep-link sub-id parsed from the hash, e.g. `#review?id=CASE-001` → `"CASE-001"`.
    /// Passed down to ReviewWorkbench and CaseTracker so they can pre-select the
    /// identified case when the page is opened via a shared link.
    let deep_link_id: yew::UseStateHandle<Option<String>> =
        use_state(|| ops_sub_id_from_hash(&initial_hash));
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let language = use_state(|| Language::Zh);

    {
        let language = *language;
        use_effect_with(language, move |language| {
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                if let Some(root) = document.document_element() {
                    let _ = root.set_attribute("lang", language.document_code());
                }
            }
            || ()
        });
    }

    // Hash routing — parse page and optional deep-link sub-id on every navigation.
    {
        let active = active.clone();
        let deep_link_id = deep_link_id.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().and_then(|window| {
                let active = active.clone();
                let deep_link_id = deep_link_id.clone();
                let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                    let hash = web_sys::window()
                        .and_then(|w| w.location().hash().ok())
                        .unwrap_or_default();
                    active.set(ops_page_from_hash(&hash));
                    deep_link_id.set(ops_sub_id_from_hash(&hash));
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

    let navigate_by_name = {
        let navigate = navigate.clone();
        Callback::from(move |name: String| {
            // Resolve page by English label; unknown names fall back to Dashboard.
            let page = OpsPage::from_label(&name).unwrap_or(OpsPage::Dashboard);
            navigate.emit(page);
        })
    };

    let toggle_language = {
        let language = language.clone();
        Callback::from(move |_: MouseEvent| language.set((*language).toggle()))
    };

    let language = *language;

    html! {
        <ContextProvider<ApiKeyContext> context={ApiKeyContext(api_key.clone())}>
        <div class="app">
            // ── Sidebar ────────────────────────────────────────────────────
            <aside class="sidebar">
                <div class="brand-block">
                    <span>{"FWA PLATFORM"}</span>
                    <h1>{tr(language, "FWA Operations", "风控运营")}</h1>
                    <p>{tr(language, "Claims FWA Operations Console", "保险理赔 FWA 运营控制台")}</p>
                </div>

                // API key input (pilot)
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

                // Grouped workflow navigation
                <nav class="module-nav" aria-label="FWA operations">
                    {for OPS_NAV_GROUPS.iter().map(|group| {
                        html! {
                            <div class="nav-section">
                                <p class="nav-section-title">{group.title_for(language)}</p>
                                {for group.pages().iter().map(|&page| {
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
                                })}
                            </div>
                        }
                    })}
                    <div class="sidebar-workflow-note">
                        <strong>{tr(language, "Workflow path", "页面路径")}</strong>
                        <span>{tr(language, "Daily Ops prioritizes intake, Investigation collects evidence and recommendations, Rules & Models governs scoring inputs, and Governance handles second-line controls.", "日常运营处理进件优先级，调查工具收集证据并形成建议，规则与模型管理评分输入，治理质控处理二线控制。")}</span>
                    </div>
                </nav>

                <div style="padding:12px 10px 8px;border-top:1px solid rgba(226,239,255,0.1);margin-top:8px;">
                    <span style="font-size:10px;color:rgba(226,239,255,0.35);display:flex;align-items:center;gap:6px;">
                        <span style="width:6px;height:6px;border-radius:50%;background:#34d399;flex-shrink:0;box-shadow:0 0 0 2px rgba(52,211,153,0.3);display:inline-block;"></span>
                        {tr(language, "Live operations", "实时运营")}
                    </span>
                </div>
            </aside>

            // ── Workspace ──────────────────────────────────────────────────
            <main class="workspace">
                <div class="workspace-topbar">
                    <div class="topbar-context">
                        <span class="eyebrow">{tr(language, "FWA Platform", "FWA 风控平台")}</span>
                        <strong>{active.label_for(language)}</strong>
                        <small>{active.description_for(language)}</small>
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
                    {match *active {
                        // ── 运营工作台 ──────────────────────────────────────
                        OpsPage::Dashboard       => html! { <OpsDashboardPage language={language} /> },
                        OpsPage::ClaimsQueue     => html! { <ClaimsQueuePage language={language} /> },
                        OpsPage::ReviewWorkbench => html! { <CaseInvestigationPage language={language} /> },
                        OpsPage::CaseTracker     => html! { <CaseTrackerPage /> },
                        // ── 调查工具（复用原有页面，完整功能）──────────────
                        OpsPage::EvidenceHub       => evidence_hub_page_with_language(navigate_by_name.clone(), language),
                        OpsPage::EvidenceRuntime   => html! { <EvidenceRuntimePage /> },
                        OpsPage::MemberProfile     => html! { <MemberProfilePage /> },
                        OpsPage::ProviderRisk      => html! { <ProviderRiskPage /> },
                        OpsPage::KnowledgeBase     => html! { <KnowledgeBasePage /> },
                        OpsPage::DataSources       => html! { <DataSourcesPage /> },
                        OpsPage::AgentInvestigator => html! { <AgentInvestigatorPage /> },
                        // ── 规则与模型（完整功能：推送审核 + 回测 + 激活）──
                        OpsPage::RuleLibrary     => html! { <RulesPage /> },
                        OpsPage::ModelGovernance => html! { <ModelsPage /> },
                        OpsPage::RoutingPolicies => html! { <RoutingPoliciesPage /> },
                        // ── 质量管理 ────────────────────────────────────────
                        OpsPage::GovernanceHub => governance_hub_page(navigate.clone(), language),
                        OpsPage::AuditSampling => html! { <AuditSamplingPage /> },
                        OpsPage::MedicalReview => html! { <MedicalReviewPage /> },
                        OpsPage::QaReview      => html! { <QaReviewPage /> },
                    }}
                </div>
            </main>
        </div>
        </ContextProvider<ApiKeyContext>>
    }
}
