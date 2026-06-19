use crate::i18n::tr;
use crate::ops_app::ops_entry_card;
use crate::ops_routing::OpsPage;
use crate::state::Language;
use yew::prelude::*;

/// Second-line governance hub: sampling QA, medical review, QA feedback,
/// and rule/model/routing-policy governance entry points.
///
/// This page does not handle claim triage — that lives in the Daily Ops section.
pub fn governance_hub_page(navigate: Callback<OpsPage>, language: Language) -> Html {
    html! {
        <section class="workflow-hub governance-hub-page">
            <div class="dashboard-header">
                <div>
                    <h2>{tr(language, "Quality & Governance", "质控与治理")}</h2>
                    <p>{tr(
                        language,
                        "Second-line controls for sampling QA, medical review, feedback closure, \
                         and rule/model/review-routing governance. This hub does not handle claim triage.",
                        "管理抽样质控、医疗复核、QA 反馈以及规则、模型和审核分流策略的发布治理；\
                         这里不承办理赔分流。",
                    )}</p>
                </div>
                <span class="status-pill">{tr(language, "Second-line controls", "二线治理")}</span>
            </div>
            <div class="workflow-card-grid governance-card-grid">
                {ops_entry_card(
                    OpsPage::AuditSampling,
                    "Quality",
                    tr(language, "Audit Sampling", "抽样审核"),
                    tr(language, "Inspect sample coverage, reviewer disagreement, and cases that need QA intervention.", "查看抽样覆盖率、复核分歧和需要质控介入的案件。"),
                    "warning",
                    &navigate,
                )}
                {ops_entry_card(
                    OpsPage::MedicalReview,
                    "Clinical",
                    tr(language, "Medical Review", "医疗复核"),
                    tr(language, "Handle medical necessity, missing evidence, and clinical reasonableness review.", "处理医疗必要性、资料缺口和临床合理性人工复核。"),
                    "strong",
                    &navigate,
                )}
                {ops_entry_card(
                    OpsPage::QaReview,
                    "Feedback",
                    tr(language, "QA Feedback", "QA 反馈"),
                    tr(language, "Close reviewer feedback into rule, model, feature, and workflow improvements.", "闭环复核意见，回流规则、模型、特征和工作流改进。"),
                    "success",
                    &navigate,
                )}
                {ops_entry_card(
                    OpsPage::RuleLibrary,
                    "Rules",
                    tr(language, "Rule Library", "规则库"),
                    tr(language, "Review pushed rules, hit performance, and backtest evidence before release.", "审核推送规则、查看命中表现和回测结果，避免静默上线。"),
                    "danger",
                    &navigate,
                )}
                {ops_entry_card(
                    OpsPage::ModelGovernance,
                    "Models",
                    tr(language, "Model Governance", "模型管理"),
                    tr(language, "Review model versions, evaluation metrics, drift, and activation decisions.", "查看模型版本、评估指标、漂移监控和激活决策。"),
                    "neutral",
                    &navigate,
                )}
                {ops_entry_card(
                    OpsPage::RoutingPolicies,
                    "Routing",
                    tr(language, "Review Routing Policies", "审核分流策略"),
                    tr(language, "Configure how risk bands route into STP, QA sampling, manual review, or rollback protection.", "设置不同风险等级进入自动通过、抽样复核、人工审核或回滚保护。"),
                    "strong",
                    &navigate,
                )}
            </div>
        </section>
    }
}
