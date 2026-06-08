pub(crate) use serde_json::{json, Value};
use wasm_bindgen::{closure::Closure, JsCast};
use yew::prelude::*;
mod api;
mod case_helpers;
mod constants;
mod data_lineage_helpers;
mod data_helpers;
mod formatting;
mod inbox_helpers;
mod i18n;
mod medical_review_helpers;
mod model_ui_helpers;
mod pages;
mod payload_helpers;
mod routing;
mod rule_helpers;
mod rule_ui_helpers;
mod runtime_helpers;
mod state;
mod types;
mod ui_helpers;
mod visual_helpers;

use api::*;
pub(crate) use case_helpers::*;
use constants::*;
pub(crate) use data_lineage_helpers::*;
pub(crate) use data_helpers::*;
pub(crate) use formatting::*;
pub(crate) use inbox_helpers::*;
use i18n::{
    apply_document_language, brand_description, module_context, module_description, module_label,
    section_label, tr,
};
pub(crate) use medical_review_helpers::*;
pub(crate) use model_ui_helpers::*;
use pages::*;
pub(crate) use payload_helpers::*;
use routing::{
    active_module_from_location, is_known_module, module_icon_class, set_module_hash,
    workspace_system_map, CONTRACT_PANELS, NAV_SECTIONS,
};
pub(crate) use rule_helpers::*;
pub(crate) use rule_ui_helpers::*;
pub(crate) use runtime_helpers::*;
use state::{ApiState, Language};
use types::*;
pub(crate) use ui_helpers::*;
pub(crate) use visual_helpers::*;

#[function_component(App)]
fn app() -> Html {
    let active = use_state(active_module_from_location);
    let language = use_state(|| Language::En);
    let select_module = {
        let active = active.clone();
        Callback::from(move |module: String| {
            if is_known_module(&module) {
                set_module_hash(&module);
                active.set(module);
            }
        })
    };
    let toggle_language = {
        let language = language.clone();
        Callback::from(move |_| language.set((*language).toggle()))
    };

    {
        let active = active.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().and_then(|window| {
                let active = active.clone();
                let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                    active.set(active_module_from_location());
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
                if let Some((window, callback)) = listener {
                    let _ = window.remove_event_listener_with_callback(
                        "hashchange",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    {
        let language = *language;
        use_effect(move || {
            apply_document_language(language.document_code());
            || ()
        });
    }

    html! {
        <div class="app">
            <aside class="sidebar">
                <div class="brand-block">
                    <span>{"NOVA FWA"}</span>
                    <h1>{"FWA Platform"}</h1>
                    <p>{brand_description(*language)}</p>
                </div>
                <nav class="module-nav" aria-label="FWA operations modules">
                    {for NAV_SECTIONS.iter().map(|(section, modules)| html! {
                        <div class="nav-section">
                            <p class="nav-section-title">{section_label(section, *language)}</p>
                            {for modules.iter().map(|module| {
                                let select_module = select_module.clone();
                                let module_name = (*module).to_string();
                                let is_active = *active == module_name;
                                html! {
                                    <button
                                        class={classes!(is_active.then_some("active"))}
                                        onclick={Callback::from(move |_| select_module.emit(module_name.clone()))}
                                    >
                                        <span class={classes!("nav-icon", module_icon_class(module))}></span>
                                        <span class="nav-copy">
                                            <span class="nav-label">{module_label(module, *language)}</span>
                                            <span class="nav-description">{module_description(module, *language)}</span>
                                        </span>
                                    </button>
                                }
                            })}
                        </div>
                    })}
                </nav>
            </aside>
            <main class="workspace">
                <div class="workspace-topbar">
                    <div class="topbar-context">
                        <span class="eyebrow">{tr(*language, "Real-time operations", "实时运营")}</span>
                        <strong>{module_context(&active, *language)}</strong>
                    </div>
                    <div class="topbar-actions">
                        <span class="api-chip status-live">{"live"}</span>
                        <span class="user-chip">{"Pilot Ops"}</span>
                        <button class="language-toggle" onclick={toggle_language}>
                            {(*language).code()}
                        </button>
                    </div>
                </div>
                {workspace_system_map(active.as_str(), select_module.clone(), *language)}
                <div class="workspace-content">
                    if *active == "Intake Ops" {
                        <ClaimInboxPage />
                    } else if *active == "Dashboard" {
                        <DashboardPage on_navigate={select_module.clone()} />
                    } else if *active == "Runtime Scoring" {
                        <RuntimeScoringPage />
                    } else if *active == "Review Workbench" {
                        {review_workbench_page(select_module.clone())}
                    } else if *active == "Bootstrap Ops" {
                        <BootstrapOpsPage />
                    } else if *active == "Discovery Review" {
                        {discovery_review_page(select_module.clone())}
                    } else if *active == "Evidence Hub" {
                        {evidence_hub_page(select_module.clone())}
                    } else if *active == "Provider Model Intake" {
                        <MlopsWorkspacePage />
                    } else if *active == "Evidence Runtime" {
                        <EvidenceRuntimePage />
                    } else if *active == "Rules" {
                        <RulesPage />
                    } else if *active == "Models" {
                        <ModelsPage />
                    } else if *active == "Routing Policies" {
                        <RoutingPoliciesPage />
                    } else if *active == "Data Sources" {
                        <DataSourcesPage />
                    } else if *active == "Factor Factory" {
                        <FactorFactoryPage />
                    } else if *active == "Leads & Cases" {
                        <LeadsCasesPage />
                    } else if *active == "Member Profile" {
                        <MemberProfilePage />
                    } else if *active == "Provider Risk" {
                        <ProviderRiskPage />
                    } else if *active == "Medical Review" {
                        <MedicalReviewPage />
                    } else if *active == "Audit Sampling" {
                        <AuditSamplingPage />
                    } else if *active == "Knowledge Base" {
                        <KnowledgeBasePage />
                    } else if *active == "Agent Investigator" {
                        <AgentInvestigatorPage />
                    } else if *active == "QA Review" {
                        <QaReviewPage />
                    } else if *active == "Governance" {
                        <GovernancePage />
                    } else {
                        <ModuleStatusPage title={(*active).clone()} />
                    }
                </div>
            </main>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ModuleStatusProps {
    title: String,
}

#[function_component(ModuleStatusPage)]
fn module_status_page(props: &ModuleStatusProps) -> Html {
    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{&props.title}</h2>
                    <p>{"This module remains part of the operations contract while the web console migrates to Yew."}</p>
                </div>
                <span class="status-pill">{"Yew shell"}</span>
            </div>
            <div class="panel">
                <h3>{"Migration Contract"}</h3>
                <p>{"Existing API, audit, QA, model, rule, and governance contracts stay in place while the console prioritizes the active operator workflow."}</p>
                <div class="tag-grid">
                    {for CONTRACT_PANELS.iter().map(|panel| html! { <span>{panel}</span> })}
                </div>
            </div>
        </section>
    }
}

fn rule_performance_for<'a>(
    performance: &'a [RulePerformance],
    rule_id: &str,
) -> Option<&'a RulePerformance> {
    performance.iter().find(|item| item.rule_id == rule_id)
}

fn main() {
    yew::Renderer::<App>::new().render();
}
