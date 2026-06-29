/// Review improvements — business-facing view of how reviewer outcomes improve
/// the FWA system. This page intentionally avoids exposing the lower-level rule
/// and model admin screens as the primary experience.
use crate::api::get_dashboard_summary;
use crate::formatting::percent_label;
use crate::i18n::tr;
use crate::ops_routing::OpsPage;
use crate::state::{use_api_key, ApiState, Language};
use crate::types::DashboardSummary;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

fn governance_metric(label: &str, value: String, note: &str, tone: &'static str) -> Html {
    html! {
        <div class={classes!("governance-metric", tone)}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{note}</small>
        </div>
    }
}

fn improvement_status(count: u32, language: Language) -> String {
    match language {
        Language::En => {
            if count == 0 {
                "Watching".to_string()
            } else {
                format!("{count} signals")
            }
        }
        Language::Zh => {
            if count == 0 {
                "观察中".to_string()
            } else {
                format!("{count} 个信号")
            }
        }
    }
}

fn improvement_item(
    title: &str,
    owner: &str,
    trigger: &str,
    evidence: &str,
    action: &str,
    count: u32,
    target: OpsPage,
    tone: &'static str,
    on_navigate: &Callback<OpsPage>,
    language: Language,
) -> Html {
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("improvement-item-card", tone)}
            onclick={Callback::from(move |_: MouseEvent| on_navigate.emit(target))}
        >
            <div class="improvement-item-main">
                <div class="improvement-item-head">
                    <strong>{title}</strong>
                    <span>{improvement_status(count, language)}</span>
                </div>
                <p>{trigger}</p>
                <small>{evidence}</small>
            </div>
            <div class="improvement-item-side">
                <span>{owner}</span>
                <b>{action}</b>
            </div>
        </button>
    }
}

fn governance_body(
    summary: &DashboardSummary,
    language: Language,
    on_navigate: &Callback<OpsPage>,
) -> Html {
    let disagreement = percent_label(summary.qa_queue.disagreement_rate);
    let false_positive = percent_label(summary.rule_governance.false_positive_rate);
    let drift = summary.model_governance.drift_detected_count;
    let breached = summary.case_sla.breached_cases;
    let unresolved = summary.qa_queue.unresolved_feedback_count;
    let rule_backlog = summary.qa_queue.rules_unresolved_feedback_count
        + summary.rule_governance.false_positive_count;
    let evidence_backlog = summary.qa_queue.features_unresolved_feedback_count
        + summary.qa_queue.provider_profile_unresolved_feedback_count
        + summary.qa_queue.workflow_unresolved_feedback_count;
    let model_backlog = summary.qa_queue.models_unresolved_feedback_count + drift;
    let calibration_backlog = summary.qa_queue.disagreement_cases;
    let total_backlog =
        calibration_backlog + rule_backlog + evidence_backlog + model_backlog + breached;

    html! {
        <div class="governance-loop-layout">
            <div class="governance-thesis">
                <div>
                    <p class="section-eyebrow">{tr(language, "Review follow-up", "复核后的改进事项")}</p>
                    <h3>{tr(language, "Do not review another case here. Decide which repeated review problem becomes system work.", "这里不处理单个案件；这里只处理复核后反复出现、需要系统侧修正的问题。")}</h3>
                </div>
                <p>{tr(
                    language,
                    "Each item below comes from reviewer disagreement, unresolved feedback, rule noise, delayed investigations, or model drift. The owner decides whether to fix policy wording, rule behavior, evidence intake, or routing signals.",
                    "下面每一项都来自复核分歧、未关闭反馈、规则噪声、超时调查或模型漂移。负责人要判断该修口径、规则、进件证据，还是分流信号。"
                )}</p>
            </div>

            <div class="governance-metric-grid">
                {governance_metric(
                    tr(language, "Review signals", "复盘信号"),
                    total_backlog.to_string(),
                    tr(language, "System-side issues from review outcomes", "复核结果沉淀出的系统侧问题"),
                    if total_backlog == 0 { "success" } else { "warning" },
                )}
                {governance_metric(
                    tr(language, "Unclosed feedback", "未关闭反馈"),
                    unresolved.to_string(),
                    tr(language, "QA feedback still needs an owner", "仍需负责人处理的 QA 反馈"),
                    if unresolved == 0 { "success" } else { "warning" },
                )}
                {governance_metric(
                    tr(language, "QA disagreement", "质检分歧"),
                    disagreement,
                    tr(language, "Cases needing calibration", "需要统一口径的案件"),
                    if summary.qa_queue.disagreement_rate <= 0.15 { "success" } else { "warning" },
                )}
                {governance_metric(
                    tr(language, "False positives", "误报率"),
                    false_positive,
                    tr(language, "Rule noise that reviewers corrected", "复核员纠正过的规则噪声"),
                    if summary.rule_governance.false_positive_rate <= 0.25 { "success" } else { "danger" },
                )}
            </div>

            <div class="improvement-worklist">
                {improvement_item(
                    tr(language, "Calibrate review policy", "统一复核口径"),
                    tr(language, "QA owner", "质检负责人"),
                    tr(language, "Reviewer decisions diverged on sampled cases. Decide whether the issue is training, unclear policy wording, or missing evidence.", "抽样案件出现复核分歧。先判断问题来自培训、政策口径不清，还是证据不足。"),
                    tr(language, "Evidence: sampled cases, QA decisions, reviewer notes", "依据：抽样案件、QA 结论、复核备注"),
                    tr(language, "Handle in queue", "去处理反馈"),
                    calibration_backlog,
                    OpsPage::ActionQueue,
                    if calibration_backlog > 0 { "warning" } else { "neutral" },
                    on_navigate,
                    language,
                )}
                {improvement_item(
                    tr(language, "Reduce rule noise", "降低规则噪声"),
                    tr(language, "Rule owner", "规则负责人"),
                    tr(language, "Reviewers corrected rule hits as false positives. Decide whether to narrow the rule, pause it, or collect more evidence before changing it.", "复核员把部分规则命中纠正为误报。判断是收窄规则、暂停规则，还是先补证据再改。"),
                    tr(language, "Evidence: rule feedback, false-positive count, reviewed hits", "依据：规则反馈、误报数量、已复核命中"),
                    tr(language, "Review rule feedback", "查看规则反馈"),
                    rule_backlog,
                    OpsPage::ActionQueue,
                    if summary.rule_governance.false_positive_rate > 0.25 { "danger" } else { "neutral" },
                    on_navigate,
                    language,
                )}
                {improvement_item(
                    tr(language, "Repair evidence intake", "修复证据进件"),
                    tr(language, "Evidence owner", "证据负责人"),
                    tr(language, "Feedback points to missing fields, provider profile gaps, or workflow friction. Fix the intake packet before reviewers keep asking for the same documents.", "反馈指向字段缺失、机构画像不足或流程摩擦。先修进件包，避免复核员反复补同类材料。"),
                    tr(language, "Evidence: feature feedback, provider-profile feedback, workflow feedback", "依据：字段反馈、机构画像反馈、流程反馈"),
                    tr(language, "Open workbench", "打开工作台"),
                    evidence_backlog,
                    OpsPage::Investigate,
                    if evidence_backlog > 0 { "warning" } else { "neutral" },
                    on_navigate,
                    language,
                )}
                {improvement_item(
                    tr(language, "Review routing signal", "复核分流信号"),
                    tr(language, "Model owner", "模型负责人"),
                    tr(language, "Model signals only route work. Drift or model feedback means the team should review whether high-risk items are still being prioritized correctly.", "模型信号只负责分流。出现漂移或模型反馈时，要复核高风险线索是否仍被正确排优先级。"),
                    tr(language, "Evidence: drift alerts, model feedback, precision and recall", "依据：漂移告警、模型反馈、precision/recall"),
                    tr(language, "Check today view", "查看今日概况"),
                    model_backlog,
                    OpsPage::Dashboard,
                    if model_backlog > 0 { "danger" } else { "neutral" },
                    on_navigate,
                    language,
                )}
                {improvement_item(
                    tr(language, "Shorten late investigations", "压缩超时调查"),
                    tr(language, "Review lead", "复核主管"),
                    tr(language, "Open cases are breaching SLA. Decide whether the blocker is missing evidence, unclear ownership, or too many low-value alerts.", "调查案件已经超时。判断阻塞来自缺证据、归属不清，还是低价值警报过多。"),
                    tr(language, "Evidence: breached cases, open investigations, triage time", "依据：超时案件、调查中案件、分诊耗时"),
                    tr(language, "Open late cases", "查看超时案件"),
                    breached,
                    OpsPage::Investigate,
                    if breached > 0 { "warning" } else { "neutral" },
                    on_navigate,
                    language,
                )}
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct SystemLearningPageProps {
    pub language: Language,
    pub on_navigate: Callback<OpsPage>,
}

#[function_component(SystemLearningPage)]
pub fn system_learning_page(props: &SystemLearningPageProps) -> Html {
    let api_key = use_api_key();
    let state = use_state(|| ApiState::<DashboardSummary>::Idle);
    let language = props.language;

    {
        let api_key = api_key.clone();
        let state = state.clone();
        use_effect_with((), move |_| {
            let api_key = (*api_key).clone();
            let state = state.clone();
            state.set(ApiState::Loading);
            spawn_local(async move {
                state.set(match get_dashboard_summary(api_key).await {
                    Ok(summary) => ApiState::Ready(summary),
                    Err(error) => ApiState::Failed(error),
                });
            });
            || ()
        });
    }

    html! {
        <div class="ops-page system-learning-page">
            <div class="ops-page-header">
                <div>
                    <h2>{tr(language, "Improvement Worklist", "改进事项")}</h2>
                    <p class="muted">{tr(language, "System-side work created by human review outcomes.", "人工复核结果沉淀出的系统侧工作。")}</p>
                </div>
            </div>

            { match &*state {
                ApiState::Idle | ApiState::Loading => html! {
                    <div class="loading-placeholder">
                        <span class="loading-pulse"/>
                        <p class="muted">{tr(language, "Loading improvement state...", "正在加载复盘状态...")}</p>
                    </div>
                },
                ApiState::Failed(error) => html! {
                    <p class="empty">{match language {
                        Language::En => format!("Load failed: {error}"),
                        Language::Zh => format!("加载失败：{error}"),
                    }}</p>
                },
                ApiState::Ready(summary) => governance_body(summary, language, &props.on_navigate),
            } }
        </div>
    }
}
