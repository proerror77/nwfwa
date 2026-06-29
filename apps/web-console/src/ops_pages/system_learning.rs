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

fn governance_action(
    title: &str,
    owner: &str,
    body: &str,
    evidence: &str,
    action: &str,
    target: OpsPage,
    tone: &'static str,
    on_navigate: &Callback<OpsPage>,
) -> Html {
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("governance-action-card", tone)}
            onclick={Callback::from(move |_: MouseEvent| on_navigate.emit(target))}
        >
            <div>
                <span>{owner}</span>
                <strong>{title}</strong>
            </div>
            <p>{body}</p>
            <small>{evidence}</small>
            <b>{action}</b>
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
    let precision = percent_label(summary.rule_governance.precision);
    let drift = summary.model_governance.drift_detected_count;
    let breached = summary.case_sla.breached_cases;

    html! {
        <div class="governance-loop-layout">
            <div class="governance-thesis">
                <div>
                    <p class="section-eyebrow">{tr(language, "When to use this page", "什么时候用这个页面")}</p>
                    <h3>{tr(language, "After reviewers finish cases, decide what the system team must fix next.", "复核员处理完案件后，用这里决定系统侧下一步要修什么。")}</h3>
                </div>
                <p>{tr(
                    language,
                    "It is not the daily case queue. It turns repeated review outcomes into follow-up work for QA, rules, evidence intake, and routing signals.",
                    "它不是每天处理单案的队列；它把重复出现的复核结果变成质检、规则、证据进件和分流信号的改进任务。"
                )}</p>
            </div>

            <div class="governance-metric-grid">
                {governance_metric(
                    tr(language, "Rule precision", "规则命中率"),
                    precision,
                    tr(language, "Confirmed cases after review", "人工复核后的确认比例"),
                    if summary.rule_governance.precision >= 0.70 { "success" } else { "warning" },
                )}
                {governance_metric(
                    tr(language, "False positives", "误报率"),
                    false_positive,
                    tr(language, "Noise reviewers must correct", "需要复核员纠正的噪声"),
                    if summary.rule_governance.false_positive_rate <= 0.25 { "success" } else { "danger" },
                )}
                {governance_metric(
                    tr(language, "QA disagreement", "质检分歧"),
                    disagreement,
                    tr(language, "Cases needing calibration", "需要统一口径的案件"),
                    if summary.qa_queue.disagreement_rate <= 0.15 { "success" } else { "warning" },
                )}
                {governance_metric(
                    tr(language, "Signal drift", "信号漂移"),
                    drift.to_string(),
                    tr(language, "Model signals needing owner review", "需要负责人查看的模型信号"),
                    if drift == 0 { "success" } else { "danger" },
                )}
            </div>

            <div class="governance-action-grid">
                {governance_action(
                    tr(language, "Close reviewer disagreement", "关闭复核分歧"),
                    tr(language, "QA owner", "质检负责人"),
                    tr(language, "Use sampled cases to decide whether the issue is reviewer training, unclear policy, or weak evidence.", "用抽样案件判断问题来自复核口径、政策不清，还是证据不足。"),
                    tr(language, "Audit output: QA decision, reviewer note, evidence refs", "审计输出：QA 结论、复核备注、证据引用"),
                    tr(language, "Open work queue", "打开处理队列"),
                    OpsPage::ActionQueue,
                    if summary.qa_queue.open_cases > 0 { "warning" } else { "neutral" },
                    on_navigate,
                )}
                {governance_action(
                    tr(language, "Promote or retire rules", "上线或下线规则"),
                    tr(language, "Rule owner", "规则负责人"),
                    tr(language, "Rules should move only when review outcomes show they reduce risk without creating too much operational noise.", "规则只有在复核结果证明能降风险且不会制造过多噪声时才应调整。"),
                    tr(language, "Audit output: rule version, backtest result, approval", "审计输出：规则版本、回测结果、审批记录"),
                    tr(language, "Review noisy leads", "查看噪声来源"),
                    OpsPage::ActionQueue,
                    if summary.rule_governance.false_positive_rate > 0.25 { "danger" } else { "neutral" },
                    on_navigate,
                )}
                {governance_action(
                    tr(language, "Watch model signals", "观察模型信号"),
                    tr(language, "Model owner", "模型负责人"),
                    tr(language, "Model scores route work, but they do not decide claims. Drift means the routing signal needs review.", "模型分数用于分流，不直接决定理赔；漂移表示分流信号需要复核。"),
                    tr(language, "Audit output: evaluation run, drift alert, promotion gate", "审计输出：评估运行、漂移告警、发布门禁"),
                    tr(language, "Open today view", "回到今日概况"),
                    OpsPage::Dashboard,
                    if drift > 0 { "danger" } else { "neutral" },
                    on_navigate,
                )}
                {governance_action(
                    tr(language, "Repair evidence gaps", "修复证据缺口"),
                    tr(language, "Evidence owner", "证据负责人"),
                    tr(language, "If reviewers keep choosing 'need more evidence', the intake packet or OCR/retrieval workflow needs correction.", "如果复核员频繁选择“需补充材料”，说明进件包或 OCR/检索流程要修正。"),
                    tr(language, "Audit output: requested document type, source ref, retrieval audit", "审计输出：补件类型、来源引用、检索审计"),
                    match language {
                        Language::En => {
                            if breached > 0 { "Open late investigations" } else { "Open investigation workbench" }
                        }
                        Language::Zh => {
                            if breached > 0 { "查看超时调查" } else { "打开调查工作台" }
                        }
                    },
                    OpsPage::Investigate,
                    if breached > 0 { "warning" } else { "neutral" },
                    on_navigate,
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
                    <h2>{tr(language, "Review Improvements", "复盘改进")}</h2>
                    <p class="muted">{tr(language, "This page exists only when review outcomes create follow-up work.", "这个页面只回答一件事：复核结果暴露了什么系统问题，下一步去哪处理。")}</p>
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
