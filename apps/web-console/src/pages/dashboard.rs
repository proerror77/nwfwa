use crate::api::*;
use crate::formatting::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use crate::visual_helpers::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DashboardPageProps {
    pub on_navigate: Callback<String>,
}

#[function_component(DashboardPage)]
pub fn dashboard_page(props: &DashboardPageProps) -> Html {
    let api_key = use_api_key();
    let summary_state = use_state(|| ApiState::<DashboardSummary>::Idle);

    let load_summary = {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_dashboard_summary(api_key).await {
                    Ok(summary) => ApiState::Ready(summary),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_summary = load_summary.clone();
        Callback::from(move |_| load_summary.emit(()))
    };

    {
        let load_summary = load_summary.clone();
        use_effect_with((), move |_| {
            load_summary.emit(());
            || ()
        });
    }

    html! {
        <section class="dashboard">
            <div class="dashboard-header">
                    <div>
                        <h2>{"Dashboard"}</h2>
                    <p>{"Watch the operating queue, risk value, review load, and governance health without exposing low-frequency integration tools."}</p>
                </div>
                <span class="status-pill">{"Pilot Operations"}</span>
            </div>

            <section class="panel">
                <h3>{"Dashboard Source"}</h3>
                <p class="empty">{"Using the configured pilot operations principal for queue, value, review-load, and governance signals."}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*summary_state, ApiState::Loading)}>
                        {if matches!(&*summary_state, ApiState::Loading) { "Refreshing..." } else { "Refresh dashboard" }}
                    </button>
                </div>
            </section>

            <DashboardView state={(*summary_state).clone()} on_navigate={props.on_navigate.clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct DashboardProps {
    state: ApiState<DashboardSummary>,
    on_navigate: Callback<String>,
}

#[function_component(DashboardView)]
fn dashboard_view(props: &DashboardProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load the dashboard to inspect operational value and governance coverage."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading dashboard summary..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(summary) => html! {
                    <>
                        {dashboard_pilot_runway(summary, &props.on_navigate)}
                        <section class="panel result-stack">
                            <h3>{"Executive KPIs"}</h3>
                            <div class="score-hero visual-kpis">
                                {kpi_card("Suspected FWA", &summary.suspected_claims.to_string(), "risk")}
                                {kpi_card("Confirmed FWA", &summary.confirmed_fwa.to_string(), "confirmed")}
                                {kpi_card("Risk Amount", &summary.risk_amount, "amount")}
                                {kpi_card("Precision at Capacity", &percent_label(summary.rule_governance.precision), "qa")}
                                {kpi_card("SLA Breach Rate", &percent_label(summary.case_sla.sla_breach_rate), "risk")}
                                {kpi_card("Review Cost vs Saving", &format!("{} / {}", summary.value_measurement.review_cost, summary.saving_amount), "saving")}
                            </div>
                            <div class="summary-grid">
                                {kpi_card("Savings", &summary.saving_amount, "saving")}
                                {kpi_card("Rule Hits", &summary.rule_hits.to_string(), "rule")}
                                {kpi_card("Investigations", &summary.investigation_results.to_string(), "case")}
                                {kpi_card("QA Reviews", &summary.qa_reviews.to_string(), "qa")}
                                <div><span>{"Risk mix"}</span><strong>{map_counts_business_label(&summary.rag_distribution)}</strong></div>
                                <div><span>{"Schemes"}</span><strong>{map_counts_business_label(&summary.scheme_distribution)}</strong></div>
                            </div>
                            {dashboard_value_proof(summary)}
                            <div class="visual-board">
                                {distribution_bars("Risk distribution", &summary.rag_distribution)}
                                {distribution_bars("Scheme mix", &summary.scheme_distribution)}
                            </div>
                            {operator_queue_snapshot(summary, &props.on_navigate)}
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn dashboard_value_proof(summary: &DashboardSummary) -> Html {
    let value = &summary.value_measurement;
    html! {
        <div class="visual-panel wide-visual value-proof-panel">
            <div class="panel-heading-row">
                <h4>{"Value proof"}</h4>
                <span class="status-token success">{"confirmed vs estimated"}</span>
            </div>
            <div class="summary-grid">
                <div>
                    <span>{"Confirmed prevented payment"}</span>
                    <strong>{format!("{} {}", value.currency, value.prevented_payment)}</strong>
                </div>
                <div>
                    <span>{"Recovered amount"}</span>
                    <strong>{format!("{} {}", value.currency, value.recovered_amount)}</strong>
                </div>
                <div>
                    <span>{"Estimated impact"}</span>
                    <strong>{format!("{} {}", value.currency, value.estimated_impact)}</strong>
                </div>
                <div>
                    <span>{"Review cost"}</span>
                    <strong>{format!("{} {}", value.currency, value.review_cost)}</strong>
                </div>
                <div>
                    <span>{"Reviewer hours"}</span>
                    <strong>{&value.reviewer_capacity_hours}</strong>
                </div>
            </div>
            <p class="empty">{&value.evidence_caveat}</p>
        </div>
    }
}

fn dashboard_pilot_runway(summary: &DashboardSummary, on_navigate: &Callback<String>) -> Html {
    let signal_label = if summary.layer_scores.is_empty() {
        "no signal evidence".into()
    } else {
        format!("{} risk signals", summary.layer_scores.len())
    };
    let qa_work = summary.qa_queue.open_cases + summary.qa_queue.unresolved_feedback_count;
    let audit_label = percent_label(summary.audit_coverage.canonical_trace_coverage);
    html! {
        <section class="panel pilot-runway-panel">
            <div class="section-header">
                <div>
                    <h3>{"Customer Pilot Proof Runway"}</h3>
                    <p>{"A one-screen path for proving a scoped customer principal can move from intake to scoring, human review, QA feedback, audit trace, cost tracking, and savings confirmation."}</p>
                </div>
                <span class="status-token strong">{"demo chain"}</span>
            </div>
            <div class="pilot-runway-map">
                <div class="runway-line"></div>
                {pilot_runway_step("Principal", "Configured principal", "actor + customer scope", "Intake Ops", "source", on_navigate)}
                {pilot_runway_step("Intake", &summary.suspected_claims.to_string(), "normalized claims", "Intake Ops", "intake", on_navigate)}
                {pilot_runway_step("Risk", &signal_label, &map_counts_business_label(&summary.rag_distribution), "Leads & Cases", "score", on_navigate)}
                {pilot_runway_step("Case", &summary.case_sla.open_cases.to_string(), "open investigations", "Leads & Cases", "case", on_navigate)}
                {pilot_runway_step("QA", &qa_work.to_string(), "open QA + feedback", "Review Workbench", "qa", on_navigate)}
                {pilot_runway_step("Audit", &audit_label, "canonical trace coverage", "Governance", "audit", on_navigate)}
                {pilot_runway_step("Value", &summary.value_measurement.prevented_payment, "confirmed prevented payment", "Dashboard", "roi", on_navigate)}
            </div>
            <div class="pilot-runway-proof">
                <div>
                    <span>{"Human gate"}</span>
                    <strong>{format!("{} cases / {} QA", summary.case_sla.open_cases, summary.qa_queue.open_cases)}</strong>
                    <small>{"High-risk work remains routed to manual review, medical review, QA, or investigation."}</small>
                </div>
                <div>
                    <span>{"Agent boundary"}</span>
                    <strong>{format!("{} evidence-backed / {} runs", summary.agent_governance.evidence_backed_runs, summary.agent_governance.total_runs)}</strong>
                    <small>{"Agent output is shown as investigation assistance, with policy checks and approvals tracked separately."}</small>
                </div>
                <div>
                    <span>{"Savings / review cost"}</span>
                    <strong>{format!("{} / {}", summary.saving_amount, summary.value_measurement.review_cost)}</strong>
                    <small>{"Costs are tracked as pilot investment until reviewed savings are confirmed."}</small>
                </div>
            </div>
        </section>
    }
}

fn pilot_runway_step(
    label: &'static str,
    value: &str,
    detail: &str,
    target: &'static str,
    tone: &'static str,
    on_navigate: &Callback<String>,
) -> Html {
    let target_name = target.to_string();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("pilot-runway-step", tone)}
            onclick={Callback::from(move |_| on_navigate.emit(target_name.clone()))}
        >
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{detail}</small>
        </button>
    }
}
