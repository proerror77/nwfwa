// Old pages are kept for reference but not rendered in the ops platform build.
// The new customer-facing OpsApp is the active renderer.
#![allow(dead_code, unused_imports, unused_variables)]
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
mod ops_app;
mod ops_pages;
mod ops_routing;
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
use constants::*;
pub(crate) use data_helpers::*;
pub(crate) use formatting::*;
use i18n::{
    apply_document_language, brand_description, module_context, module_description, module_label,
    section_label, setup_translations, tr,
};
use ops_app::OpsApp;
use pages::*;
use routing::{
    active_module_from_location, module_from_name, set_module_hash, workspace_system_map, Module,
    NAV_SECTIONS,
};
pub(crate) use rule_helpers::*;
use state::{ApiKeyContext, ApiState, Language};
use types::*;
pub(crate) use ui_helpers::*;
pub(crate) use visual_helpers::*;

#[function_component(App)]
fn app() -> Html {
    let active = use_state(active_module_from_location);
    let language = use_state(|| Language::En);
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let select_module = {
        let active = active.clone();
        Callback::from(move |module: Module| {
            set_module_hash(module);
            active.set(module);
        })
    };
    let select_module_name = {
        let select_module = select_module.clone();
        Callback::from(move |module: String| {
            if let Some(module) = module_from_name(&module) {
                select_module.emit(module);
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
        <ContextProvider<ApiKeyContext> context={ApiKeyContext(api_key)}>
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
                                let module = *module;
                                let is_active = *active == module;
                                html! {
                                    <button
                                        class={classes!(is_active.then_some("active"))}
                                        onclick={Callback::from(move |_| select_module.emit(module))}
                                    >
                                        <span class={classes!("nav-icon", module.icon_class())}></span>
                                        <span class="nav-copy">
                                            <span class="nav-label">{module_label(module.as_str(), *language)}</span>
                                            <span class="nav-description">{module_description(module.as_str(), *language)}</span>
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
                        <strong>{module_context(active.as_str(), *language)}</strong>
                    </div>
                    <div class="topbar-actions">
                        <span class="api-chip status-live">{"live"}</span>
                        <span class="user-chip">{"Pilot Ops"}</span>
                        <button class="language-toggle" onclick={toggle_language}>
                            {(*language).code()}
                        </button>
                    </div>
                </div>
                {workspace_system_map(*active, select_module.clone(), *language)}
                <div class="workspace-content">
                    {match *active {
                        Module::IntakeOps => html! { <ClaimInboxPage /> },
                        Module::Dashboard => html! { <DashboardPage on_navigate={select_module_name.clone()} /> },
                        Module::RuntimeScoring => html! { <RuntimeScoringPage /> },
                        Module::ReviewWorkbench => review_workbench_page(select_module_name.clone()),
                        Module::BootstrapOps => html! { <BootstrapOpsPage /> },
                        Module::DiscoveryReview => discovery_review_page(select_module_name.clone()),
                        Module::EvidenceHub => evidence_hub_page(select_module_name.clone()),
                        Module::ProviderModelIntake => html! { <MlopsWorkspacePage /> },
                        Module::EvidenceRuntime => html! { <EvidenceRuntimePage /> },
                        Module::Rules => html! { <RulesPage /> },
                        Module::Models => html! { <ModelsPage /> },
                        Module::RoutingPolicies => html! { <RoutingPoliciesPage /> },
                        Module::DataSources => html! { <DataSourcesPage /> },
                        Module::FactorFactory => html! { <FactorFactoryPage /> },
                        Module::LeadsCases => html! { <LeadsCasesPage /> },
                        Module::MemberProfile => html! { <MemberProfilePage /> },
                        Module::ProviderRisk => html! { <ProviderRiskPage /> },
                        Module::MedicalReview => html! { <MedicalReviewPage /> },
                        Module::AuditSampling => html! { <AuditSamplingPage /> },
                        Module::KnowledgeBase => html! { <KnowledgeBasePage /> },
                        Module::AgentInvestigator => html! { <AgentInvestigatorPage /> },
                        Module::QaReview => html! { <QaReviewPage /> },
                        Module::Governance => html! { <GovernancePage /> },
                    }}
                </div>
            </main>
        </div>
        </ContextProvider<ApiKeyContext>>
    }
}

fn rule_performance_for<'a>(
    performance: &'a [RulePerformance],
    rule_id: &str,
) -> Option<&'a RulePerformance> {
    performance.iter().find(|item| item.rule_id == rule_id)
}

fn main() {
    setup_translations();
    yew::Renderer::<OpsApp>::new().render();
}
