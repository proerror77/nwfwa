use crate::{
    optional_number, percent_label, percent_width, provider_signal_row, ratio, scaled_width,
    status_tone, ModelOpsSnapshot, ModelPerformance, ModelPromotionGates, ModelRetrainingReadiness,
    ModelVersion,
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
                    <h3>{"模型发布检查表"}</h3>
                    <p>{"任何模型影响案件路由前，先确认版本、漂移、影子评估、QA 标签、发布门禁和阻塞项。"}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.gates.decision))}>{&snapshot.gates.decision}</span>
            </div>
            <div class="model-monitoring-cockpit">
                <aside class="model-monitoring-brief">
                    <span class="eyebrow">{"候选模型"}</span>
                    <strong>{&model_label}</strong>
                    <dl>
                        <div><dt>{"运行时"}</dt><dd>{active_model.map(|model| model.runtime_kind.as_str()).unwrap_or("运行时待定")}</dd></div>
                        <div><dt>{"提供方"}</dt><dd>{active_model.map(|model| model.execution_provider.as_str()).unwrap_or("提供方待定")}</dd></div>
                        <div><dt>{"复核模式"}</dt><dd>{active_model.map(|model| model.review_mode.as_str()).unwrap_or("复核待定")}</dd></div>
                        <div><dt>{"最新评估"}</dt><dd>{&snapshot.gates.latest_evaluation_id}</dd></div>
                    </dl>
                </aside>

                {model_release_workbench(snapshot, active_model, &model_label, &psi_label, gate_ratio, label_ratio)}

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

fn model_release_workbench(
    snapshot: &ModelOpsSnapshot,
    active_model: Option<&ModelVersion>,
    model_label: &str,
    psi_label: &str,
    gate_ratio: f64,
    label_ratio: f64,
) -> Html {
    let artifact_status = snapshot
        .gates
        .artifact_evidence
        .rust_serving_status
        .as_deref()
        .or(snapshot
            .gates
            .artifact_evidence
            .serving_manifest_uri
            .as_deref())
        .unwrap_or("待确认");
    let latency_status = snapshot
        .gates
        .artifact_evidence
        .rust_serving_latency_status
        .as_deref()
        .unwrap_or("待确认");
    let blocker_label = snapshot
        .gates
        .blockers
        .first()
        .map(String::as_str)
        .or_else(|| snapshot.retraining.blockers.first().map(String::as_str))
        .unwrap_or("无阻塞项");
    let trigger_label = if snapshot.retraining.retraining_triggers.is_empty() {
        "暂无触发项".to_string()
    } else {
        snapshot.retraining.retraining_triggers.join(" / ")
    };

    html! {
        <div class="model-release-workbench">
            <div class="model-release-head">
                <span>{"发布检查工作台"}</span>
                <strong>{format!("候选模型：{model_label}")}</strong>
            </div>
            <div class="model-release-grid">
                {model_release_card(
                    "候选版本",
                    model_label.to_string(),
                    "客户评估和试点安装时看到的具体版本。",
                    "version",
                    vec![
                        ("版本", active_model.map(|model| model.version.clone()).unwrap_or_else(|| "待定".into())),
                        ("运行时", active_model.map(|model| model.runtime_kind.clone()).unwrap_or_else(|| "待定".into())),
                        ("提供方", active_model.map(|model| model.execution_provider.clone()).unwrap_or_else(|| "待定".into())),
                        ("复核模式", active_model.map(|model| model.review_mode.clone()).unwrap_or_else(|| "待定".into())),
                    ],
                )}
                {model_release_card(
                    "漂移与路由",
                    format!("{} / PSI {}", snapshot.performance.drift_status, psi_label),
                    "确认模型分数分布稳定后，才允许影响案件路由。",
                    if snapshot.performance.drift_status == "stable" { "stable" } else { "watch" },
                    vec![
                        ("评分样本", snapshot.performance.scored_runs.to_string()),
                        ("平均风险分", format!("{:.1}", snapshot.performance.average_score)),
                        ("高风险命中", snapshot.performance.high_risk_count.to_string()),
                        ("路由决策", snapshot.gates.decision.clone()),
                    ],
                )}
                {model_release_card(
                    "影子评估",
                    snapshot.gates.latest_evaluation_id.clone(),
                    "用影子评估和数据质量证明新模型不会误伤复核流程。",
                    "shadow",
                    vec![
                        ("门禁进度", format!("{}/{} 通过", snapshot.gates.passed_count, snapshot.gates.total_count)),
                        ("门禁比例", percent_label(gate_ratio)),
                        ("数据质量", snapshot.gates.source_data_quality_status.clone()),
                        ("服务证据", artifact_status.to_string()),
                    ],
                )}
                {model_release_card(
                    "QA 标签",
                    format!("{} 已批准", snapshot.gates.approved_label_count),
                    "把人工复核标签作为客户判断模型是否可发布的依据。",
                    "labels",
                    vec![
                        ("标签准备度", percent_label(label_ratio)),
                        ("开放反馈", snapshot.gates.unresolved_model_feedback_count.to_string()),
                        ("待复核标签", snapshot.retraining.needs_review_label_count.to_string()),
                        ("阻塞项", blocker_label.to_string()),
                    ],
                )}
                {model_release_card(
                    "重训准备",
                    snapshot.retraining.recommendation.clone(),
                    "如果暂不发布，明确下一版小模型该用哪些反馈继续训练。",
                    "train",
                    vec![
                        ("漂移状态", snapshot.retraining.drift_status.clone()),
                        ("开放反馈", snapshot.retraining.open_model_feedback_count.to_string()),
                        ("触发原因", trigger_label),
                        ("延迟状态", latency_status.to_string()),
                    ],
                )}
            </div>
        </div>
    }
}

fn model_release_card(
    title: &'static str,
    metric: String,
    caption: &'static str,
    tone: &'static str,
    items: Vec<(&'static str, String)>,
) -> Html {
    html! {
        <details class={classes!("model-release-card", tone)}>
            <summary>
                <span>{title}</span>
                <strong>{metric}</strong>
                <small>{caption}</small>
            </summary>
            <div class="model-release-detail">
                {for items.into_iter().map(|(label, value)| html! {
                    <div class="model-release-row">
                        <span>{label}</span>
                        <strong>{value}</strong>
                    </div>
                })}
            </div>
        </details>
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
