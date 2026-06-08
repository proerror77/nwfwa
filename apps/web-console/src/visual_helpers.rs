use crate::{
    business_label, icon_class, map_counts_business_label, percent_label, scaled_width,
    DashboardSummary,
};
use std::collections::BTreeMap;
use yew::prelude::*;

pub(crate) fn workflow_action_card(
    title: &str,
    description: &str,
    command: &str,
    target: &str,
    tone: &str,
    on_navigate: &Callback<String>,
) -> Html {
    let target = target.to_string();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("workflow-action-card", tone.to_string())}
            onclick={Callback::from(move |_| on_navigate.emit(target.clone()))}
        >
            <span>{title}</span>
            <strong>{command}</strong>
            <small>{description}</small>
        </button>
    }
}

pub(crate) fn kpi_card(label: &str, value: &str, icon: &str) -> Html {
    html! {
        <div class="visual-kpi">
            <span class={classes!("visual-icon", icon_class(icon))}></span>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

pub(crate) fn operator_queue_snapshot(
    summary: &DashboardSummary,
    on_navigate: &Callback<String>,
) -> Html {
    html! {
        <div class="visual-panel wide-visual operator-queue-panel">
            <div class="panel-heading-row">
                <h4>{"Next actions"}</h4>
                <span class="status-token strong">{"click to work"}</span>
            </div>
            <div class="operator-queue">
                {operator_queue_card("Triage", &summary.suspected_claims.to_string(), "suspected leads", "Leads & Cases", "danger", on_navigate)}
                {operator_queue_card("Investigate", &summary.case_sla.open_cases.to_string(), "open cases", "Leads & Cases", "warning", on_navigate)}
                {operator_queue_card("Review", &summary.qa_queue.open_cases.to_string(), "open QA samples", "Review Workbench", "strong", on_navigate)}
                {operator_queue_card("Govern", &percent_label(summary.audit_coverage.canonical_trace_coverage), "trace coverage", "Governance", "success", on_navigate)}
            </div>
            {dashboard_operations_map(summary)}
        </div>
    }
}

fn operator_queue_card(
    action: &str,
    value: &str,
    metric: &str,
    target: &str,
    tone: &str,
    on_navigate: &Callback<String>,
) -> Html {
    let target = target.to_string();
    let target_label = target.clone();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("operator-queue-card", tone.to_string())}
            onclick={Callback::from(move |_| on_navigate.emit(target.clone()))}
        >
            <span>{action}</span>
            <strong>{value}</strong>
            <small>{metric}</small>
            <em>{target_label}</em>
        </button>
    }
}

fn dashboard_operations_map(summary: &DashboardSummary) -> Html {
    let review_label = format!(
        "{} cases / {} QA",
        summary.case_sla.open_cases, summary.qa_queue.open_cases
    );
    let engine_label = format!(
        "rules + risk mix: {} / {}",
        summary.rule_hits,
        map_counts_business_label(&summary.rag_distribution)
    );
    html! {
        <div class="ops-system-map-shell">
            <div class="panel-heading-row compact-heading-row">
                <h4>{"FWA operating map"}</h4>
                <span class="status-token strong">{"PRD runtime topology"}</span>
            </div>
            <div class="ops-system-map">
                {ops_map_node("TPA", "claim intake", &summary.suspected_claims.to_string(), "source")}
                <div class="ops-map-core">
                    <span>{"Detect"}</span>
                    <strong>{"Risk scoring service"}</strong>
                    <small>{engine_label}</small>
                </div>
                {ops_map_node("Review", "human queue", &review_label, "qa")}
                {ops_map_node("Evidence", "assistive pack", &format!("{} runs", summary.agent_governance.total_runs), "agent")}
                {ops_map_node("Audit", "trace + approval", &percent_label(summary.audit_coverage.canonical_trace_coverage), "audit")}
                {ops_map_node("Savings", "confirmation gate", &summary.saving_amount, "roi")}
            </div>
        </div>
    }
}

fn ops_map_node(label: &str, caption: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("ops-map-node", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}

pub(crate) fn risk_node(layer: &str, label: &str, value: &str, caption: &str) -> Html {
    html! {
        <div class="risk-node">
            <span class="risk-node-badge">{layer}</span>
            <strong>{value}</strong>
            <span>{label}</span>
            <small>{caption}</small>
        </div>
    }
}

pub(crate) fn distribution_bars(title: &str, counts: &BTreeMap<String, u32>) -> Html {
    if counts.is_empty() {
        return html! {
            <div class="visual-panel">
                <h4>{title}</h4>
                <p class="empty">{"No distribution records."}</p>
            </div>
        };
    }
    let max_count = counts.values().copied().max().unwrap_or(1);
    html! {
        <div class="visual-panel">
            <h4>{title}</h4>
            <div class="bar-stack">
                {for counts.iter().map(|(label, count)| {
                    let width = scaled_width(*count, max_count);
                    html! {
                        <div class="bar-row">
                            <span>{business_label(label)}</span>
                            <div class="bar-track"><i style={format!("width: {width};")}></i></div>
                            <strong>{count}</strong>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}
