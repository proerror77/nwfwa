use crate::{
    optional_number, percent_label, percent_width, provider_signal_row, ratio, scaled_width,
    status_tone, ModelOpsSnapshot, ModelPerformance, ModelPromotionGates, ModelRetrainingReadiness,
};
use yew::prelude::*;

pub(crate) fn model_telemetry_visual(
    performance: &ModelPerformance,
    gates: &ModelPromotionGates,
    retraining: &ModelRetrainingReadiness,
) -> Html {
    let high_risk_density = if performance.scored_runs == 0 {
        0.0
    } else {
        performance.high_risk_count as f64 / performance.scored_runs as f64
    };
    let score_level = (performance.average_score / 100.0).clamp(0.0, 1.0);
    let psi_level = performance.score_psi.unwrap_or(0.0).clamp(0.0, 1.0);
    html! {
        <div class="visual-board model-telemetry">
            <div class="visual-panel">
                <h4>{"Model telemetry map"}</h4>
                <div class="telemetry-orbit">
                    <div class="orbit-core">
                        <strong>{format!("{:.1}", performance.average_score)}</strong>
                        <span>{"avg score"}</span>
                    </div>
                    {telemetry_node("score", score_level, "Score")}
                    {telemetry_node("density", high_risk_density, "High risk")}
                    {telemetry_node("psi", psi_level, "PSI")}
                    {telemetry_node("gates", ratio(gates.passed_count as u32, gates.total_count as u32), "Gates")}
                </div>
            </div>
            <div class="visual-panel">
                <h4>{"Retraining control"}</h4>
                <div class="bar-stack">
                    {meter_row("Approved labels", gates.approved_label_count as u32, 100)}
                    {meter_row("Open feedback", retraining.open_model_feedback_count, 20)}
                    {meter_row("Needs review", retraining.needs_review_label_count, 20)}
                </div>
                <div class="status-ribbon">
                    <span>{format!("drift: {}", retraining.drift_status)}</span>
                    <strong>{&retraining.recommendation}</strong>
                </div>
            </div>
        </div>
    }
}

pub(crate) fn model_monitoring_cockpit(snapshot: &ModelOpsSnapshot) -> Html {
    let active_model = snapshot
        .models
        .iter()
        .find(|model| model.status == "active")
        .or_else(|| snapshot.models.first());
    let model_label = active_model
        .map(|model| format!("{} {}", model.model_key, model.version))
        .unwrap_or_else(|| snapshot.performance.model_key.clone());
    let gate_ratio = ratio(
        snapshot.gates.passed_count as u32,
        snapshot.gates.total_count as u32,
    );
    let label_ratio = ratio(snapshot.gates.approved_label_count, 100);
    let psi_label = optional_number(snapshot.performance.score_psi);
    let first_blocker = snapshot
        .gates
        .blockers
        .first()
        .map(String::as_str)
        .or_else(|| snapshot.retraining.blockers.first().map(String::as_str))
        .unwrap_or("no blocker");

    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Model Monitoring Cockpit"}</h3>
                    <p>{"A pilot-facing view of model version, drift, shadow evidence, promotion gates, QA labels, and retraining readiness before any model affects routing."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.gates.decision))}>{&snapshot.gates.decision}</span>
            </div>
            <div class="model-monitoring-cockpit">
                <aside class="model-monitoring-brief">
                    <span class="eyebrow">{"Active candidate"}</span>
                    <strong>{model_label}</strong>
                    <dl>
                        <div><dt>{"Runtime"}</dt><dd>{active_model.map(|model| model.runtime_kind.as_str()).unwrap_or("runtime pending")}</dd></div>
                        <div><dt>{"Provider"}</dt><dd>{active_model.map(|model| model.execution_provider.as_str()).unwrap_or("provider pending")}</dd></div>
                        <div><dt>{"Review mode"}</dt><dd>{active_model.map(|model| model.review_mode.as_str()).unwrap_or("review pending")}</dd></div>
                        <div><dt>{"Latest eval"}</dt><dd>{&snapshot.gates.latest_evaluation_id}</dd></div>
                    </dl>
                </aside>

                <div class="model-monitoring-map">
                    <div class="model-monitoring-link horizontal"></div>
                    <div class="model-monitoring-link diagonal-a"></div>
                    <div class="model-monitoring-link diagonal-b"></div>
                    <div class="model-monitoring-core">
                        <span>{"Release gate"}</span>
                        <strong>{&snapshot.performance.model_key}</strong>
                    </div>
                    {model_monitoring_node("Version lock", active_model.map(|model| model.version.as_str()).unwrap_or("pending"), "top", "version")}
                    {model_monitoring_node("Drift watch", &format!("{} / PSI {}", snapshot.performance.drift_status, psi_label), "right", "drift")}
                    {model_monitoring_node("Shadow evidence", &snapshot.gates.latest_evaluation_id, "bottom", "shadow")}
                    {model_monitoring_node("QA labels", &format!("{} approved", snapshot.gates.approved_label_count), "left", "labels")}
                    {model_monitoring_node("Retraining", &snapshot.retraining.recommendation, "lower-right", "train")}
                </div>

                <aside class="model-monitoring-actions">
                    <span class="eyebrow">{"Promotion readiness"}</span>
                    <div class="model-monitoring-meter">
                        <span>{"Gate pass"}</span>
                        <div><i style={format!("width: {};", percent_width(gate_ratio))}></i></div>
                        <strong>{percent_label(gate_ratio)}</strong>
                    </div>
                    <div class="model-monitoring-meter">
                        <span>{"Label readiness"}</span>
                        <div><i style={format!("width: {};", percent_width(label_ratio))}></i></div>
                        <strong>{snapshot.gates.approved_label_count}</strong>
                    </div>
                    <div class="provider-signal-stack">
                        {provider_signal_row("Data quality", &snapshot.gates.source_data_quality_status, "strong")}
                        {provider_signal_row("Drift", &snapshot.retraining.drift_status, "warning")}
                        {provider_signal_row("Open feedback", &snapshot.retraining.open_model_feedback_count.to_string(), "neutral")}
                        {provider_signal_row("Blocker", first_blocker, "danger")}
                    </div>
                </aside>
            </div>
        </section>
    }
}

fn model_monitoring_node(label: &str, value: &str, position: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("model-monitoring-node", position.to_string(), tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn telemetry_node(kind: &str, value: f64, label: &str) -> Html {
    html! {
        <div class={classes!("orbit-node", kind.to_string())}>
            <span>{label}</span>
            <strong>{percent_label(value)}</strong>
        </div>
    }
}

fn meter_row(label: &str, value: u32, max_value: u32) -> Html {
    html! {
        <div class="bar-row">
            <span>{label}</span>
            <div class="bar-track"><i style={format!("width: {};", scaled_width(value, max_value.max(1)))}></i></div>
            <strong>{value}</strong>
        </div>
    }
}
