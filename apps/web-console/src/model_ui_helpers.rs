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
                <h4>{"模型遥测图"}</h4>
                <div class="telemetry-orbit">
                    <div class="orbit-core">
                        <strong>{format!("{:.1}", performance.average_score)}</strong>
                        <span>{"平均分"}</span>
                    </div>
                    {telemetry_node("score", score_level, "评分")}
                    {telemetry_node("density", high_risk_density, "高风险")}
                    {telemetry_node("psi", psi_level, "PSI")}
                    {telemetry_node("gates", ratio(gates.passed_count as u32, gates.total_count as u32), "门禁")}
                </div>
            </div>
            <div class="visual-panel">
                <h4>{"重训控制"}</h4>
                <div class="bar-stack">
                    {meter_row("已批准标签", gates.approved_label_count as u32, 100)}
                    {meter_row("开放反馈", retraining.open_model_feedback_count, 20)}
                    {meter_row("待复核", retraining.needs_review_label_count, 20)}
                </div>
                <div class="status-ribbon">
                    <span>{format!("漂移：{}", retraining.drift_status)}</span>
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
        .unwrap_or("无阻塞项");

    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"模型监控驾驶舱"}</h3>
                    <p>{"面向试点演示的模型版本、漂移、影子证据、发布门禁、QA 标签和重训准备度视图，任何模型影响路由前都先在这里确认。"}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.gates.decision))}>{&snapshot.gates.decision}</span>
            </div>
            <div class="model-monitoring-cockpit">
                <aside class="model-monitoring-brief">
                    <span class="eyebrow">{"当前候选模型"}</span>
                    <strong>{model_label}</strong>
                    <dl>
                        <div><dt>{"运行时"}</dt><dd>{active_model.map(|model| model.runtime_kind.as_str()).unwrap_or("运行时待定")}</dd></div>
                        <div><dt>{"提供方"}</dt><dd>{active_model.map(|model| model.execution_provider.as_str()).unwrap_or("提供方待定")}</dd></div>
                        <div><dt>{"复核模式"}</dt><dd>{active_model.map(|model| model.review_mode.as_str()).unwrap_or("复核待定")}</dd></div>
                        <div><dt>{"最新评估"}</dt><dd>{&snapshot.gates.latest_evaluation_id}</dd></div>
                    </dl>
                </aside>

                <div class="model-monitoring-map">
                    <div class="model-monitoring-link horizontal"></div>
                    <div class="model-monitoring-link diagonal-a"></div>
                    <div class="model-monitoring-link diagonal-b"></div>
                    <div class="model-monitoring-core">
                        <span>{"发布门禁"}</span>
                        <strong>{&snapshot.performance.model_key}</strong>
                    </div>
                    {model_monitoring_node("版本锁定", active_model.map(|model| model.version.as_str()).unwrap_or("待定"), "top", "version")}
                    {model_monitoring_node("漂移监控", &format!("{} / PSI {}", snapshot.performance.drift_status, psi_label), "right", "drift")}
                    {model_monitoring_node("影子证据", &snapshot.gates.latest_evaluation_id, "bottom", "shadow")}
                    {model_monitoring_node("QA 标签", &format!("{} 已批准", snapshot.gates.approved_label_count), "left", "labels")}
                    {model_monitoring_node("重训", &snapshot.retraining.recommendation, "lower-right", "train")}
                </div>

                <aside class="model-monitoring-actions">
                    <span class="eyebrow">{"发布准备度"}</span>
                    <div class="model-monitoring-meter">
                        <span>{"门禁通过"}</span>
                        <div><i style={format!("width: {};", percent_width(gate_ratio))}></i></div>
                        <strong>{percent_label(gate_ratio)}</strong>
                    </div>
                    <div class="model-monitoring-meter">
                        <span>{"标签准备度"}</span>
                        <div><i style={format!("width: {};", percent_width(label_ratio))}></i></div>
                        <strong>{snapshot.gates.approved_label_count}</strong>
                    </div>
                    <div class="provider-signal-stack">
                        {provider_signal_row("数据质量", &snapshot.gates.source_data_quality_status, "strong")}
                        {provider_signal_row("漂移", &snapshot.retraining.drift_status, "warning")}
                        {provider_signal_row("开放反馈", &snapshot.retraining.open_model_feedback_count.to_string(), "neutral")}
                        {provider_signal_row("阻塞项", first_blocker, "danger")}
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
