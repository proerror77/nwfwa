use serde_json::{json, Value};
use std::collections::BTreeMap;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::HtmlInputElement;
use yew::prelude::*;
mod api;
mod constants;
mod data_helpers;
mod formatting;
mod inbox_helpers;
mod i18n;
mod model_ui_helpers;
mod pages;
mod routing;
mod rule_helpers;
mod rule_ui_helpers;
mod runtime_helpers;
mod state;
mod types;
mod visual_helpers;

use api::*;
use constants::*;
pub(crate) use data_helpers::*;
pub(crate) use formatting::*;
pub(crate) use inbox_helpers::*;
use i18n::{
    apply_document_language, brand_description, module_context, module_description, module_label,
    section_label, tr,
};
pub(crate) use model_ui_helpers::*;
use pages::*;
use routing::{
    active_module_from_location, is_known_module, module_icon_class, set_module_hash,
    workspace_system_map, CONTRACT_PANELS, NAV_SECTIONS,
};
pub(crate) use rule_helpers::*;
pub(crate) use rule_ui_helpers::*;
pub(crate) use runtime_helpers::*;
use state::{ApiState, Language};
use types::*;
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

fn timeline_item(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("timeline-item", status_tone(tone))}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn case_action(label: &str, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("case-action", tone.to_string())}>
            <strong>{label}</strong>
            <span>{caption}</span>
        </div>
    }
}

fn scaled_width(value: u32, max_value: u32) -> String {
    let width = if max_value == 0 {
        0.0
    } else {
        value as f64 / max_value as f64 * 100.0
    };
    format!("{:.0}%", width.clamp(4.0, 100.0))
}

fn percent_width(value: f64) -> String {
    format!("{:.0}%", (value * 100.0).clamp(4.0, 100.0))
}

fn ratio(value: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64
    }
}

fn icon_class(icon: &str) -> &'static str {
    match icon {
        "risk" => "icon-risk",
        "confirmed" => "icon-confirmed",
        "amount" => "icon-amount",
        "saving" => "icon-saving",
        "rule" => "icon-rule",
        "case" => "icon-case",
        "qa" => "icon-qa-card",
        "currency" => "icon-currency",
        _ => "icon-default",
    }
}

fn agent_investigation_payload(
    claim_id: String,
    risk_score: String,
    rag: String,
    scheme_family: String,
    top_reasons: String,
    diagnosis_code: String,
    provider_region: String,
    tags: String,
) -> Result<Value, String> {
    let top_reasons = parse_tags(&top_reasons);
    let tags = parse_tags(&tags);
    if claim_id.trim().is_empty() {
        return Err("claim id is required".into());
    }
    if !matches!(rag.trim(), "GREEN" | "AMBER" | "RED") {
        return Err("RAG must be GREEN, AMBER, or RED".into());
    }
    if top_reasons.is_empty() {
        return Err("at least one top reason is required".into());
    }
    if diagnosis_code.trim().is_empty() || provider_region.trim().is_empty() || tags.is_empty() {
        return Err("diagnosis code, provider region, and at least one tag are required".into());
    }
    let scheme_family = scheme_family.trim();
    Ok(json!({
        "claim_id": claim_id.trim(),
        "risk_score": parse_risk_score(&risk_score)?,
        "rag": rag.trim(),
        "scheme_family": if scheme_family.is_empty() {
            Value::Null
        } else {
            Value::String(scheme_family.to_string())
        },
        "top_reasons": top_reasons,
        "similar_case_query": {
            "diagnosis_code": diagnosis_code.trim(),
            "provider_region": provider_region.trim(),
            "tags": tags
        }
    }))
}

fn audit_sample_payload(
    sample_mode: String,
    population_definition: String,
    inclusion_criteria: String,
    sample_size: String,
    reviewer: String,
    assignment_queue: String,
    deterministic_seed: String,
) -> Result<Value, String> {
    let sample_mode = sample_mode.trim();
    if !matches!(
        sample_mode,
        "risk_ranked" | "random_control" | "stratified" | "post_payment_audit" | "qa_calibration"
    ) {
        return Err("sample mode must be risk_ranked, random_control, stratified, post_payment_audit, or qa_calibration".into());
    }
    if population_definition.trim().is_empty() {
        return Err("population definition is required".into());
    }
    if reviewer.trim().is_empty() || assignment_queue.trim().is_empty() {
        return Err("reviewer and assignment queue are required".into());
    }
    let sample_size = sample_size
        .trim()
        .parse::<usize>()
        .map_err(|error| format!("sample size must be a positive integer: {error}"))?;
    if sample_size == 0 {
        return Err("sample size must be greater than zero".into());
    }
    let inclusion_criteria = serde_json::from_str::<Value>(&inclusion_criteria)
        .map_err(|error| format!("inclusion criteria JSON is invalid: {error}"))?;
    if !inclusion_criteria.is_object() {
        return Err("inclusion criteria must be a JSON object".into());
    }
    let deterministic_seed = deterministic_seed.trim();
    Ok(json!({
        "sample_mode": sample_mode,
        "population_definition": population_definition.trim(),
        "inclusion_criteria": inclusion_criteria,
        "sample_size": sample_size,
        "reviewer": reviewer.trim(),
        "assignment_queue": assignment_queue.trim(),
        "deterministic_seed": if deterministic_seed.is_empty() {
            Value::Null
        } else {
            Value::String(deterministic_seed.to_string())
        }
    }))
}

fn total_dataset_rows(datasets: &[DatasetRecord]) -> String {
    datasets
        .iter()
        .map(|dataset| dataset.row_count)
        .sum::<u64>()
        .to_string()
}

fn total_schema_fields(datasets: &[DatasetRecord]) -> usize {
    datasets.iter().map(|dataset| dataset.fields.len()).sum()
}

fn total_field_mappings(datasets: &[DatasetRecord]) -> usize {
    datasets.iter().map(|dataset| dataset.mappings.len()).sum()
}

fn latest_dataset(datasets: &[DatasetRecord]) -> Option<&DatasetRecord> {
    datasets
        .iter()
        .max_by_key(|dataset| (&dataset.dataset_key, &dataset.dataset_version))
}

fn dataset_version_label(dataset: &DatasetRecord) -> String {
    format!("{}:{}", dataset.dataset_key, dataset.dataset_version)
}

fn health_for_dataset<'a>(
    health: &'a [DatasetHealthRecord],
    dataset_id: &str,
) -> Option<&'a DatasetHealthRecord> {
    health.iter().find(|item| item.dataset_id == dataset_id)
}

fn status_tone(status: &str) -> &'static str {
    let normalized = status.to_ascii_lowercase();
    if normalized.contains("fail")
        || normalized.contains("error")
        || normalized.contains("breach")
        || normalized.contains("blocked")
        || normalized.contains("high")
    {
        "danger"
    } else if normalized.contains("warn")
        || normalized.contains("pending")
        || normalized.contains("review")
        || normalized.contains("medium")
    {
        "warning"
    } else if normalized.contains("ready")
        || normalized.contains("active")
        || normalized.contains("ok")
        || normalized.contains("pass")
        || normalized.contains("good")
    {
        "success"
    } else {
        "neutral"
    }
}

fn lineage_for<'a>(
    lineage: &'a [ModelEvaluationLineageRecord],
    evaluation_run_id: &str,
) -> Option<&'a ModelEvaluationLineageRecord> {
    lineage
        .iter()
        .find(|record| record.evaluation_run_id == evaluation_run_id)
}

fn lineage_data_quality_label(lineage: Option<&ModelEvaluationLineageRecord>) -> String {
    lineage
        .map(|record| {
            format!(
                "{} / {}",
                record
                    .source_data_quality_status
                    .as_deref()
                    .unwrap_or("missing"),
                optional_number(record.source_data_quality_score)
            )
        })
        .unwrap_or_else(|| "missing".into())
}

fn lineage_source_label(lineage: Option<&ModelEvaluationLineageRecord>) -> String {
    lineage
        .map(|record| {
            format!(
                "{}:{} / {} / {} {}",
                record.source_dataset_key.as_deref().unwrap_or("missing"),
                record
                    .source_dataset_version
                    .as_deref()
                    .unwrap_or("missing"),
                record.source_dataset_id.as_deref().unwrap_or("missing"),
                record.model_key,
                record.model_version
            )
        })
        .unwrap_or_else(|| "missing".into())
}

fn rule_performance_for<'a>(
    performance: &'a [RulePerformance],
    rule_id: &str,
) -> Option<&'a RulePerformance> {
    performance.iter().find(|item| item.rule_id == rule_id)
}

fn selected_lead<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    selected_lead_id: &str,
) -> Option<&'a LeadRecord> {
    let selected_lead_id = selected_lead_id.trim();
    if selected_lead_id.is_empty() {
        snapshot.leads.first()
    } else {
        snapshot
            .leads
            .iter()
            .find(|lead| lead.lead_id == selected_lead_id)
    }
}

fn selected_case<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    selected_case_id: &str,
) -> Option<&'a CaseRecord> {
    let selected_case_id = selected_case_id.trim();
    if selected_case_id.is_empty() {
        snapshot.cases.first()
    } else {
        snapshot
            .cases
            .iter()
            .find(|case| case.case_id == selected_case_id)
    }
}

fn latest_lead_for_score<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    claim_id: &str,
    score_run_id: &str,
) -> Option<&'a LeadRecord> {
    snapshot
        .leads
        .iter()
        .find(|lead| lead.claim_id == claim_id && lead.run_id == score_run_id)
        .or_else(|| snapshot.leads.iter().find(|lead| lead.claim_id == claim_id))
}

fn lead_for_case<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    case: &CaseRecord,
) -> Option<&'a LeadRecord> {
    snapshot
        .leads
        .iter()
        .find(|lead| lead.lead_id == case.lead_id)
}

fn live_tpa_demo_payload(summary: &DashboardSummary) -> Result<Value, String> {
    let suffix = format!(
        "{}-{}-{}",
        summary.suspected_claims, summary.confirmed_fwa, summary.rule_hits
    );
    let mut payload = serde_json::from_str::<Value>(LIVE_TPA_DEMO_PAYLOAD)
        .map_err(|error| format!("live demo payload JSON is invalid: {error}"))?;
    payload["transNo"] = Value::String(format!("TPA-LIVE-DEMO-{suffix}"));
    payload["reportCase"]["reportNo"] = Value::String(format!("CLM-LIVE-DEMO-{suffix}"));
    Ok(payload)
}

fn selected_medical_item<'a>(
    items: &'a [MedicalReviewQueueItem],
    selected_audit_id: &str,
) -> Option<&'a MedicalReviewQueueItem> {
    let selected_audit_id = selected_audit_id.trim();
    if selected_audit_id.is_empty() {
        items.first()
    } else {
        items.iter().find(|item| item.audit_id == selected_audit_id)
    }
}

fn refs_or_fallback(refs_text: &str, fallback: Vec<String>) -> Vec<String> {
    let refs = parse_tags(refs_text);
    if refs.is_empty() {
        fallback
            .into_iter()
            .filter(|reference| !reference.trim().is_empty())
            .collect()
    } else {
        refs
    }
}

fn medical_review_cockpit(items: &[MedicalReviewQueueItem]) -> Html {
    let Some(item) = items.first() else {
        return html! {};
    };
    let missing_evidence = item
        .missing_evidence
        .first()
        .map(String::as_str)
        .unwrap_or("none");
    let canonical_source = item
        .canonical_source_refs
        .first()
        .map(String::as_str)
        .unwrap_or("source pending");
    let canonical_evidence = item
        .canonical_evidence_refs
        .first()
        .map(String::as_str)
        .unwrap_or("evidence pending");
    let first_item = item.first_item_code.as_deref().unwrap_or("item pending");
    let first_issue = item.first_issue_type.as_deref().unwrap_or("issue pending");
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Clinical evidence cockpit"}</h3>
                    <p>{"Clinical reasonableness workbench linking diagnosis support, bill item evidence, missing records, reviewer outcome, and audit trace."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&item.evidence_status))}>{business_label(&item.evidence_status)}</span>
            </div>
            <div class="clinical-cockpit">
                <aside class="case-brief clinical-brief">
                    <span>{"Selected review"}</span>
                    <strong>{&item.claim_id}</strong>
                    <dl>
                        <div><dt>{"Audit"}</dt><dd>{&item.audit_id}</dd></div>
                        <div><dt>{"Route"}</dt><dd>{business_label(&item.review_route)}</dd></div>
                        <div><dt>{"Status"}</dt><dd>{business_label(&item.review_status)}</dd></div>
                        <div><dt>{"Score"}</dt><dd>{item.medical_reasonableness_score}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("findings {}", item.item_finding_count)}</span>
                        <span>{format!("missing {}", item.missing_evidence.len())}</span>
                        <span>{format!("refs {}", item.evidence_refs.len() + item.canonical_evidence_refs.len())}</span>
                    </div>
                </aside>

                <div class="clinical-evidence-map">
                    <div class="clinical-map-title">
                        <span>{"Medical necessity path"}</span>
                        <strong>{format!("{} -> {}", first_item, first_issue)}</strong>
                    </div>
                    <div class="clinical-path-line"></div>
                    <div class="clinical-node diagnosis">
                        <span>{"Diagnosis"}</span>
                        <strong>{canonical_source}</strong>
                    </div>
                    <div class="clinical-node item">
                        <span>{"Bill item"}</span>
                        <strong>{first_item}</strong>
                    </div>
                    <div class="clinical-node record">
                        <span>{"Medical record"}</span>
                        <strong>{canonical_evidence}</strong>
                    </div>
                    <div class="clinical-node gap">
                        <span>{"Evidence gap"}</span>
                        <strong>{missing_evidence}</strong>
                    </div>
                    <div class="clinical-node reviewer">
                        <span>{"Reviewer"}</span>
                        <strong>{item.reviewer.as_deref().unwrap_or("pending")}</strong>
                    </div>
                </div>

                <aside class="case-timeline clinical-timeline">
                    <h4>{"Clinical trace"}</h4>
                    {timeline_item("Queue created", item.created_at.as_deref().unwrap_or("pending"), "done")}
                    {timeline_item("Evidence status", &business_label(&item.evidence_status), &item.evidence_status)}
                    {timeline_item("Review decision", &item.review_decision.as_deref().map(business_label).unwrap_or_else(|| "Pending".into()), item.review_decision.as_deref().unwrap_or("pending"))}
                    {timeline_item("Review audit", item.review_audit_id.as_deref().unwrap_or("pending"), "pending")}
                </aside>
            </div>
            <div class="clinical-outcome-grid">
                <h4>{"Controlled outcomes"}</h4>
                {case_action("Documentation issue", "clinical evidence incomplete", "warning")}
                {case_action("Medical necessity review required", "human medical gate", "strong")}
                {case_action("Insufficient evidence", "request supplement", "neutral")}
                {case_action("Medical necessity issue", "manual action only", "danger")}
                {case_action("Clinical evidence sufficient", "close clinical gap", "strong")}
                {case_action("False positive", "requires audit note", "neutral")}
            </div>
        </section>
    }
}

fn medical_review_fallback_refs(item: &MedicalReviewQueueItem) -> Vec<String> {
    let mut refs = item.evidence_refs.clone();
    refs.extend(item.canonical_evidence_refs.clone());
    refs.push(format!("audit:{}", item.audit_id));
    refs.into_iter().fold(Vec::new(), |mut values, value| {
        if !values.contains(&value) {
            values.push(value);
        }
        values
    })
}

fn data_lineage_cockpit(snapshot: &DataSourcesSnapshot) -> Html {
    let source_count = unique_dataset_sources(&snapshot.datasets);
    let canonical_count = unique_canonical_targets(&snapshot.datasets);
    let feature_count = feature_mapping_count(&snapshot.datasets);
    let online_ready = snapshot
        .health
        .iter()
        .map(|health| health.online_ready_count)
        .sum::<u32>();
    let issue_count = snapshot
        .health
        .iter()
        .map(|health| health.issue_count)
        .sum::<u32>();
    let quality_label = data_quality_summary(&snapshot.health);
    let quality_tone = status_tone(&quality_label);
    let source_label = snapshot
        .datasets
        .first()
        .map(|dataset| format!("{} / {}", dataset.source_key, dataset.storage_format))
        .unwrap_or_else(|| "no source registered".into());
    let canonical_label = first_canonical_target(&snapshot.datasets);
    let feature_label = first_feature_mapping(&snapshot.datasets);
    let model_label = snapshot
        .evaluations
        .first()
        .map(|evaluation| format!("{} {}", evaluation.model_key, evaluation.model_version))
        .unwrap_or_else(|| "no evaluation".into());
    let runtime_label = if online_ready > 0 {
        format!("{} online fields", online_ready)
    } else {
        "not online ready".into()
    };
    let audit_label = if issue_count == 0 {
        "no open data issues".into()
    } else {
        format!("{} data issues", issue_count)
    };

    html! {
        <section class="panel data-lineage-cockpit">
            <div class="section-header">
                <div>
                    <h3>{"Data Lineage Cockpit"}</h3>
                    <p>{"A visual control map for how external datasets become governed features, model evaluation evidence, scoring inputs, and audit records."}</p>
                </div>
                <span class={classes!("status-token", quality_tone)}>{quality_label.clone()}</span>
            </div>
            <div class="data-lineage-map" aria-label="Data lineage flow">
                <div class="lineage-rail rail-a"></div>
                <div class="lineage-rail rail-b"></div>
                <div class="lineage-rail rail-c"></div>
                {data_lineage_node("source", "Sources", &source_count.to_string(), &source_label)}
                {data_lineage_node("contract", "Schema contract", &total_schema_fields(&snapshot.datasets).to_string(), "field profiles and split manifests")}
                {data_lineage_node("canonical", "Canonical map", &canonical_count.to_string(), &canonical_label)}
                {data_lineage_node("feature", "Feature ready", &feature_count.to_string(), &feature_label)}
                {data_lineage_node("model", "Model lineage", &snapshot.evaluations.len().to_string(), &model_label)}
                {data_lineage_node("runtime", "Runtime inputs", &online_ready.to_string(), &runtime_label)}
                {data_lineage_node("audit", "Audit guard", &issue_count.to_string(), &audit_label)}
            </div>
            <div class="data-lineage-proof-grid">
                <div>
                    <span>{"Governed contract"}</span>
                    <strong>{format!("{} datasets / {} mappings", snapshot.datasets.len(), total_field_mappings(&snapshot.datasets))}</strong>
                    <small>{"schema hash, profile URI, manifest URI, and split records remain visible before scoring."}</small>
                </div>
                <div>
                    <span>{"Evaluation evidence"}</span>
                    <strong>{format!("{} runs / {}", snapshot.evaluations.len(), lineage_source_coverage(&snapshot.lineage))}</strong>
                    <small>{"model metrics stay tied to source dataset version and data-quality state."}</small>
                </div>
                <div>
                    <span>{"Pilot blocker signal"}</span>
                    <strong>{audit_label}</strong>
                    <small>{"data health issues are shown as readiness evidence, not hidden behind model output."}</small>
                </div>
            </div>
        </section>
    }
}

fn data_lineage_node(tone: &'static str, label: &'static str, value: &str, detail: &str) -> Html {
    html! {
        <div class={classes!("data-lineage-node", tone)}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{detail}</small>
        </div>
    }
}

fn unique_dataset_sources(datasets: &[DatasetRecord]) -> usize {
    datasets
        .iter()
        .fold(Vec::<&str>::new(), |mut values, dataset| {
            if !values.contains(&dataset.source_key.as_str()) {
                values.push(dataset.source_key.as_str());
            }
            values
        })
        .len()
}

fn unique_canonical_targets(datasets: &[DatasetRecord]) -> usize {
    datasets
        .iter()
        .flat_map(|dataset| dataset.mappings.iter())
        .fold(Vec::<&str>::new(), |mut values, mapping| {
            if !values.contains(&mapping.canonical_target.as_str()) {
                values.push(mapping.canonical_target.as_str());
            }
            values
        })
        .len()
}

fn feature_mapping_count(datasets: &[DatasetRecord]) -> usize {
    datasets
        .iter()
        .flat_map(|dataset| dataset.mappings.iter())
        .filter(|mapping| mapping.feature_name.is_some())
        .count()
}

fn first_canonical_target(datasets: &[DatasetRecord]) -> String {
    datasets
        .iter()
        .flat_map(|dataset| dataset.mappings.iter())
        .next()
        .map(|mapping| mapping.canonical_target.clone())
        .unwrap_or_else(|| "no canonical mapping".into())
}

fn first_feature_mapping(datasets: &[DatasetRecord]) -> String {
    datasets
        .iter()
        .flat_map(|dataset| dataset.mappings.iter())
        .find_map(|mapping| mapping.feature_name.clone())
        .unwrap_or_else(|| "no feature mapping".into())
}

fn data_quality_summary(health: &[DatasetHealthRecord]) -> String {
    if health.is_empty() {
        return "no health record".into();
    }
    if health
        .iter()
        .any(|item| status_tone(&item.data_quality_status) == "danger")
    {
        return "data blocker".into();
    }
    if health
        .iter()
        .any(|item| status_tone(&item.data_quality_status) == "warning")
    {
        return "review required".into();
    }
    "data ready".into()
}

fn lineage_source_coverage(lineage: &[ModelEvaluationLineageRecord]) -> String {
    let covered = lineage
        .iter()
        .filter(|record| record.source_dataset_id.is_some())
        .count();
    format!("{} source-linked", covered)
}

fn open_lead_count(leads: &[LeadRecord]) -> usize {
    leads
        .iter()
        .filter(|lead| !matches!(lead.status.as_str(), "closed" | "rejected"))
        .count()
}

fn active_case_count(cases: &[CaseRecord]) -> usize {
    cases
        .iter()
        .filter(|case| !matches!(case.status.as_str(), "closed" | "rejected"))
        .count()
}

fn breached_case_count(cases: &[CaseRecord]) -> usize {
    cases
        .iter()
        .filter(|case| case.sla_status == "breached")
        .count()
}

fn lead_status_count(leads: &[LeadRecord], status: &str) -> usize {
    leads.iter().filter(|lead| lead.status == status).count()
}

fn case_status_count(cases: &[CaseRecord], status: &str) -> usize {
    cases.iter().filter(|case| case.status == status).count()
}

fn queue_meter(label: &str, value: usize, total: usize, tone: &str) -> Html {
    let width = if total == 0 {
        "0%".to_string()
    } else {
        percent_width(value as f64 / total as f64)
    };
    html! {
        <div class={classes!("queue-meter", tone.to_string())}>
            <div>
                <span>{label}</span>
                <strong>{value}</strong>
            </div>
            <i><b style={format!("width: {width};")}></b></i>
        </div>
    }
}

fn top_scheme_label(leads: &[LeadRecord]) -> String {
    let mut counts = BTreeMap::new();
    for lead in leads {
        *counts.entry(lead.scheme_family.as_str()).or_insert(0_usize) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(scheme, count)| format!("{} ({})", readable_token(scheme), count))
        .unwrap_or_else(|| "No active pattern".into())
}

fn lead_stage_label(status: &str) -> String {
    match status {
        "new" => "New lead".into(),
        "pending_evidence" => "Needs evidence".into(),
        "triaged" => "Case opened".into(),
        "closed" => "Closed".into(),
        other => readable_token(other),
    }
}

fn lead_stage_tone(status: &str) -> &'static str {
    match status {
        "pending_evidence" => "danger",
        "new" => "warning",
        "triaged" | "closed" => "success",
        _ => "neutral",
    }
}

fn case_stage_label(status: &str) -> String {
    match status {
        "investigating" => "Investigating",
        "pending_evidence" => "Waiting evidence",
        "confirmed" => "Confirmed",
        "closed" => "Closed",
        "rejected" => "Rejected",
        "triage" => "Triage",
        other => return readable_token(other),
    }
    .into()
}

fn case_stage_tone(status: &str) -> &'static str {
    match status {
        "investigating" | "pending_evidence" | "triage" => "warning",
        "confirmed" | "closed" => "success",
        "rejected" => "neutral",
        _ => "neutral",
    }
}

fn priority_label(priority: &str) -> String {
    match priority {
        "high" => "High priority",
        "medium" => "Medium priority",
        "low" => "Low priority",
        other => return readable_token(other),
    }
    .into()
}

fn priority_tone(priority: &str) -> &'static str {
    match priority {
        "high" => "danger",
        "medium" => "warning",
        "low" => "neutral",
        _ => "strong",
    }
}

fn sla_label(status: &str) -> &'static str {
    match status {
        "breached" => "Over SLA",
        "on_track" => "On track",
        _ => "SLA pending",
    }
}

fn sla_tone(status: &str) -> &'static str {
    match status {
        "breached" => "danger",
        "on_track" => "success",
        _ => "neutral",
    }
}

fn routing_review_modes(policies: &[RoutingPolicyRecord]) -> String {
    count_by(policies.iter().map(|policy| policy.review_mode.as_str()))
}

fn count_by<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0_u32) += 1;
    }
    map_counts_label(&counts)
}

fn average_medical_score(items: &[MedicalReviewQueueItem]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    let total = items
        .iter()
        .map(|item| item.medical_reasonableness_score as u32)
        .sum::<u32>();
    total as f64 / items.len() as f64
}

fn text_input(label: &'static str, state: &UseStateHandle<String>) -> Html {
    html! {
        <label>
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

fn approval_summary(approvals: &[AgentApprovalView]) -> String {
    if approvals.is_empty() {
        return "none".into();
    }
    approvals
        .iter()
        .map(|approval| {
            format!(
                "{} {}:{} by {} at {} evidence={} reason={}",
                approval.approval_id,
                approval.proposed_action,
                approval.decision,
                approval.approver,
                approval.created_at.as_deref().unwrap_or("unknown"),
                approval.evidence_refs.len(),
                approval.reason
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn approval_count_label(approvals: &[AgentApprovalView]) -> String {
    if approvals.is_empty() {
        "none".into()
    } else {
        format!("{} approval records", approvals.len())
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
