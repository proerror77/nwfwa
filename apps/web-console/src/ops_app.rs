use crate::constants::API_KEY_DEFAULT;
use crate::i18n::tr;
use crate::ops_pages::*;
use crate::ops_routing::{ops_page_from_hash, ops_set_hash, OpsPage, OPS_NAV_GROUPS};
use crate::pages::*;
use crate::state::{ApiKeyContext, Language};
use wasm_bindgen::{closure::Closure, JsCast};
use yew::prelude::*;

fn ops_entry_card(
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

fn governance_hub_page(navigate: Callback<OpsPage>, language: Language) -> Html {
    html! {
        <section class="workflow-hub governance-hub-page">
            <div class="dashboard-header">
                <div>
                    <h2>{tr(language, "Quality & Governance", "质控与治理")}</h2>
                    <p>{tr(language, "Second-line controls for sampling QA, medical review, feedback closure, and rule/model/review-routing governance. This hub does not handle claim triage.", "管理抽样质控、医疗复核、QA 反馈以及规则、模型和审核分流策略的发布治理；这里不承办理赔分流。")}</p>
                </div>
                <span class="status-pill">{tr(language, "Second-line controls", "二线治理")}</span>
            </div>
            <div class="workflow-card-grid governance-card-grid">
                {ops_entry_card(OpsPage::AuditSampling, "Quality", tr(language, "Audit Sampling", "抽样审核"), tr(language, "Inspect sample coverage, reviewer disagreement, and cases that need QA intervention.", "查看抽样覆盖率、复核分歧和需要质控介入的案件。"), "warning", &navigate)}
                {ops_entry_card(OpsPage::MedicalReview, "Clinical", tr(language, "Medical Review", "医疗复核"), tr(language, "Handle medical necessity, missing evidence, and clinical reasonableness review.", "处理医疗必要性、资料缺口和临床合理性人工复核。"), "strong", &navigate)}
                {ops_entry_card(OpsPage::QaReview, "Feedback", tr(language, "QA Feedback", "QA 反馈"), tr(language, "Close reviewer feedback into rule, model, feature, and workflow improvements.", "闭环复核意见，回流规则、模型、特征和工作流改进。"), "success", &navigate)}
                {ops_entry_card(OpsPage::RuleLibrary, "Rules", tr(language, "Rule Library", "规则库"), tr(language, "Review pushed rules, hit performance, and backtest evidence before release.", "审核推送规则、查看命中表现和回测结果，避免静默上线。"), "danger", &navigate)}
                {ops_entry_card(OpsPage::ModelGovernance, "Models", tr(language, "Model Governance", "模型管理"), tr(language, "Review model versions, evaluation metrics, drift, and activation decisions.", "查看模型版本、评估指标、漂移监控和激活决策。"), "neutral", &navigate)}
                {ops_entry_card(OpsPage::RoutingPolicies, "Routing", tr(language, "Review Routing Policies", "审核分流策略"), tr(language, "Configure how risk bands route into STP, QA sampling, manual review, or rollback protection.", "设置不同风险等级进入自动通过、抽样复核、人工审核或回滚保护。"), "strong", &navigate)}
            </div>
        </section>
    }
}

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
            // Map old module names to new OpsPage for cross-page links
            let page = match name.as_str() {
                "Member Profile" => OpsPage::MemberProfile,
                "Provider Risk" => OpsPage::ProviderRisk,
                "Knowledge Base" => OpsPage::KnowledgeBase,
                "Evidence Runtime" => OpsPage::EvidenceRuntime,
                "Data Sources" => OpsPage::DataSources,
                "Medical Review" => OpsPage::MedicalReview,
                "QA Review" => OpsPage::QaReview,
                "Agent Investigator" => OpsPage::AgentInvestigator,
                "Rules" => OpsPage::RuleLibrary,
                "Models" => OpsPage::ModelGovernance,
                "Routing Policies" => OpsPage::RoutingPolicies,
                "Case Tracker" => OpsPage::CaseTracker,
                "Governance" => OpsPage::GovernanceHub,
                "Dashboard" => OpsPage::Dashboard,
                _ => OpsPage::Dashboard,
            };
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
