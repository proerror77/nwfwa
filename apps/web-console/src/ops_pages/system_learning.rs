/// Learning and release — business-facing view of how reviewer outcomes become
/// candidate rules, model versions, and customer-approved releases.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LearningAction {
    RuleDiscovery,
    RuleValidation,
    ModelTraining,
    ReleaseDecision,
}

fn action_status(count: u32, language: Language) -> String {
    match language {
        Language::En => {
            if count == 0 {
                "Ready".to_string()
            } else {
                format!("{count} signals")
            }
        }
        Language::Zh => {
            if count == 0 {
                "可查看".to_string()
            } else {
                format!("{count} 个信号")
            }
        }
    }
}

fn learning_action_card(
    title: &str,
    actor: &str,
    trigger: &str,
    action: &str,
    count: u32,
    tone: &'static str,
    active: bool,
    selected: LearningAction,
    on_select: &Callback<LearningAction>,
    language: Language,
) -> Html {
    let on_select = on_select.clone();
    html! {
        <button
            class={classes!("learning-action-card", tone, active.then_some("active"))}
            onclick={Callback::from(move |_: MouseEvent| on_select.emit(selected))}
        >
            <div class="learning-action-main">
                <div class="learning-action-head">
                    <strong>{title}</strong>
                    <span>{action_status(count, language)}</span>
                </div>
                <p>{trigger}</p>
            </div>
            <div class="learning-action-side">
                <span>{actor}</span>
                <b>{action}</b>
            </div>
        </button>
    }
}

fn action_detail(
    summary: &DashboardSummary,
    action: LearningAction,
    language: Language,
    on_navigate: &Callback<OpsPage>,
) -> Html {
    let route_button = |label: &str, target: OpsPage| {
        let on_navigate = on_navigate.clone();
        html! {
            <button class="learning-detail-route" onclick={Callback::from(move |_: MouseEvent| on_navigate.emit(target))}>
                {label}
            </button>
        }
    };

    let approved_labels = summary.label_pool.approved_for_training;
    let needs_review_labels = summary.label_pool.needs_review;
    let rule_signals = summary.label_pool.rule_feedback
        + summary.rule_governance.false_positive_count
        + summary.rule_governance.confirmed_fwa_count;
    let model_signals = summary.label_pool.model_feedback
        + summary.model_governance.drift_detected_count
        + approved_labels;

    let (title, goal, input, output, customer, primary) = match action {
        LearningAction::RuleDiscovery => (
            tr(language, "Mine candidate rules", "挖掘候选规则"),
            tr(language, "Find explainable FWA patterns from confirmed cases, reviewer corrections, and feature importance.", "从已确认欺诈、复核纠错和特征贡献中找出可解释的 FWA 规则。"),
            match language {
                Language::En => format!("{rule_signals} rule signals, {approved_labels} approved labels"),
                Language::Zh => format!("{rule_signals} 个规则信号，{approved_labels} 个可训练标签"),
            },
            tr(language, "Candidate rule with reason, conditions, coverage, and expected false-positive risk.", "候选规则：包含原因、条件、覆盖范围和预期误报风险。"),
            tr(language, "Customer should see why this rule exists before it can enter review.", "客户先看这条规则为什么存在，再决定是否进入审核。"),
            route_button(tr(language, "Open rule discovery workbench", "打开规则挖掘台"), OpsPage::RuleLab),
        ),
        LearningAction::RuleValidation => (
            tr(language, "Validate rule version", "验证规则版本"),
            tr(language, "Run backtest and shadow checks before a candidate rule becomes part of the governed rule pack.", "候选规则进入规则包前，先跑回测和影子验证。"),
            match language {
                Language::En => format!("{} false positives, {} reviewed hits", summary.rule_governance.false_positive_count, summary.rule_governance.reviewed_count),
                Language::Zh => format!("{} 个误报，{} 个已复核命中", summary.rule_governance.false_positive_count, summary.rule_governance.reviewed_count),
            },
            tr(language, "Rule version package with backtest result, shadow result, blockers, and rollback note.", "规则版本包：回测结果、影子验证、阻塞项和回滚说明。"),
            tr(language, "Customer decides whether the rule is safe enough for their policy and tolerance.", "客户判断这条规则是否符合自己的政策和误报容忍度。"),
            route_button(tr(language, "Open rule gates", "打开规则回测与门禁"), OpsPage::RuleLab),
        ),
        LearningAction::ModelTraining => (
            tr(language, "Train candidate model", "训练候选模型"),
            tr(language, "Use approved labels and drift feedback to prepare a small model version for one FWA pattern or review mode.", "用已批准标签和漂移反馈，为某个 FWA 模式或复核模式准备一个小模型版本。"),
            match language {
                Language::En => format!("{approved_labels} approved labels, {needs_review_labels} labels still need review"),
                Language::Zh => format!("{approved_labels} 个已批准标签，{needs_review_labels} 个标签待复核"),
            },
            tr(language, "Candidate model with dataset version, feature list, precision/recall, threshold, and drift evidence.", "候选模型：包含数据集版本、特征列表、precision/recall、阈值和漂移证据。"),
            tr(language, "Customer should compare the new model against the current production behavior before installing it.", "客户需要把新模型和当前生产表现对比后，再决定是否安装。"),
            route_button(tr(language, "Open model governance", "打开模型治理"), OpsPage::ModelGovernance),
        ),
        LearningAction::ReleaseDecision => (
            tr(language, "Customer release decision", "客户发布决策"),
            tr(language, "Package candidate rules and model versions into a controlled release proposal.", "把候选规则和候选模型打包成一个受控发布提案。"),
            match language {
                Language::En => format!("{} active rules, {} evaluated models, {} model signals", summary.rule_governance.active_rules, summary.model_governance.evaluated_models, model_signals),
                Language::Zh => format!("{} 条活跃规则，{} 个已评估模型，{} 个模型信号", summary.rule_governance.active_rules, summary.model_governance.evaluated_models, model_signals),
            },
            tr(language, "Release proposal with expected benefit, risk, affected workflow, approval record, and rollback path.", "发布提案：预期收益、风险、影响流程、审批记录和回滚路径。"),
            tr(language, "Customer chooses install, pilot, reject, or defer. The system records the decision as audit evidence.", "客户选择安装、试点、拒绝或暂缓；系统把决定作为审计证据记录。"),
            route_button(tr(language, "Open release gates", "查看发布门禁"), OpsPage::ModelGovernance),
        ),
    };

    html! {
        <section class="learning-detail-panel">
            <div>
                <p class="section-eyebrow">{tr(language, "Selected business action", "当前业务动作")}</p>
                <h3>{title}</h3>
                <p>{goal}</p>
            </div>
            <div class="learning-detail-grid">
                <div><span>{tr(language, "Input", "输入")}</span><strong>{input}</strong></div>
                <div><span>{tr(language, "Output", "产出")}</span><strong>{output}</strong></div>
                <div><span>{tr(language, "Customer decision", "客户判断")}</span><strong>{customer}</strong></div>
            </div>
            <div class="learning-detail-actions">
                {primary}
            </div>
        </section>
    }
}

fn governance_body(
    summary: &DashboardSummary,
    language: Language,
    on_navigate: &Callback<OpsPage>,
    selected_action: LearningAction,
    on_select_action: &Callback<LearningAction>,
) -> Html {
    let disagreement = percent_label(summary.qa_queue.disagreement_rate);
    let false_positive = percent_label(summary.rule_governance.false_positive_rate);
    let drift = summary.model_governance.drift_detected_count;
    let approved_labels = summary.label_pool.approved_for_training;
    let rule_signals = summary.label_pool.rule_feedback
        + summary.rule_governance.false_positive_count
        + summary.rule_governance.confirmed_fwa_count;
    let model_signals =
        summary.label_pool.model_feedback + drift + summary.label_pool.approved_for_training;
    let release_signals =
        summary.rule_governance.active_rules + summary.model_governance.evaluated_models;

    html! {
        <div class="governance-loop-layout">
            <div class="governance-thesis">
                <div>
                    <p class="section-eyebrow">{tr(language, "Learning loop", "算法学习闭环")}</p>
                    <h3>{tr(language, "Reviewer outcomes become candidate rules, model versions, and customer-approved releases.", "复核结果会沉淀成候选规则、候选模型版本，再交给客户判断是否发布。")}</h3>
                </div>
                <p>{tr(
                    language,
                    "This page should not be a generic improvement list. Its job is to show how FWA learns from data: mine rules, validate rule versions, train candidate models, and prepare a release decision for the customer.",
                    "这个页面不应该是泛泛的改进列表。它要说明 FWA 如何从数据里学习：挖掘规则、验证规则版本、训练候选模型，并准备给客户审批的发布方案。"
                )}</p>
            </div>

            <div class="governance-metric-grid">
                {governance_metric(
                    tr(language, "Rule signals", "规则信号"),
                    rule_signals.to_string(),
                    tr(language, "Confirmed cases and corrections that can become rules", "可转成规则的确认案件和纠错信号"),
                    if rule_signals == 0 { "success" } else { "warning" },
                )}
                {governance_metric(
                    tr(language, "Training labels", "训练标签"),
                    approved_labels.to_string(),
                    tr(language, "Approved human labels available for model work", "可用于模型训练的人工标签"),
                    if approved_labels > 0 { "success" } else { "warning" },
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

            <div class="learning-action-grid">
                {learning_action_card(
                    tr(language, "Mine candidate rules", "挖掘候选规则"),
                    tr(language, "Rule analyst", "规则分析"),
                    tr(language, "Use confirmed fraud, reviewer corrections, and feature importance to discover explainable policy-like rules.", "用已确认欺诈、复核纠错和特征贡献，挖掘可解释、像政策条款一样能说明白的候选规则。"),
                    tr(language, "Select", "查看动作"),
                    rule_signals,
                    if rule_signals > 0 { "warning" } else { "neutral" },
                    selected_action == LearningAction::RuleDiscovery,
                    LearningAction::RuleDiscovery,
                    on_select_action,
                    language,
                )}
                {learning_action_card(
                    tr(language, "Validate rule version", "验证规则版本"),
                    tr(language, "Rule owner", "规则负责人"),
                    tr(language, "Before a rule is proposed to a customer, compare backtest, shadow-run output, false positives, and rollback risk.", "规则交给客户前，先对比回测、影子运行、误报和回滚风险。"),
                    tr(language, "Select", "查看动作"),
                    summary.rule_governance.false_positive_count,
                    if summary.rule_governance.false_positive_rate > 0.25 { "danger" } else { "neutral" },
                    selected_action == LearningAction::RuleValidation,
                    LearningAction::RuleValidation,
                    on_select_action,
                    language,
                )}
                {learning_action_card(
                    tr(language, "Train candidate model", "训练候选模型"),
                    tr(language, "Model owner", "模型负责人"),
                    tr(language, "Use approved review labels and drift feedback to train a small versioned detector for one FWA pattern.", "用已批准复核标签和漂移反馈，为某个 FWA 模式训练一个小的、可版本化的识别模型。"),
                    tr(language, "Select", "查看动作"),
                    model_signals,
                    if drift > 0 { "danger" } else { "neutral" },
                    selected_action == LearningAction::ModelTraining,
                    LearningAction::ModelTraining,
                    on_select_action,
                    language,
                )}
                {learning_action_card(
                    tr(language, "Prepare customer release", "准备客户发布决策"),
                    tr(language, "Customer owner", "客户负责人"),
                    tr(language, "Package candidate rule and model versions so the customer can choose install, pilot, reject, or defer.", "把候选规则和模型版本打包，让客户选择安装、试点、拒绝或暂缓。"),
                    tr(language, "Select", "查看动作"),
                    release_signals,
                    "neutral",
                    selected_action == LearningAction::ReleaseDecision,
                    LearningAction::ReleaseDecision,
                    on_select_action,
                    language,
                )}
            </div>

            {action_detail(summary, selected_action, language, on_navigate)}
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
    let selected_action = use_state(|| LearningAction::RuleDiscovery);
    let language = props.language;
    let on_select_action = {
        let selected_action = selected_action.clone();
        Callback::from(move |action| selected_action.set(action))
    };

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
                    <h2>{tr(language, "Learning and Release", "学习与发布")}</h2>
                    <p class="muted">{tr(language, "Turn review data into candidate rules, model versions, and customer decisions.", "把复核数据转成候选规则、模型版本和客户发布决策。")}</p>
                </div>
            </div>

            { match &*state {
                ApiState::Idle | ApiState::Loading => html! {
                    <div class="loading-placeholder">
                        <span class="loading-pulse"/>
                        <p class="muted">{tr(language, "Loading learning state...", "正在加载学习状态...")}</p>
                    </div>
                },
                ApiState::Failed(error) => html! {
                    <p class="empty">{match language {
                        Language::En => format!("Load failed: {error}"),
                        Language::Zh => format!("加载失败：{error}"),
                    }}</p>
                },
                ApiState::Ready(summary) => governance_body(
                    summary,
                    language,
                    &props.on_navigate,
                    *selected_action,
                    &on_select_action,
                ),
            } }
        </div>
    }
}
