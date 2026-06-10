use crate::visual_helpers::*;
use crate::inbox_helpers::*;
use yew::prelude::*;

pub fn review_workbench_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Review Workbench"}</h2>
                    <p>{"Use this as the single entry point for human review. Clinical necessity and QA feedback stay separate, but operators do not need two top-level menus."}</p>
                </div>
                <span class="status-pill">{"Human review"}</span>
            </div>
            <div class="workflow-card-grid">
                {workflow_action_card("Medical Review", "Resolve clinical reasonableness, necessity, and documentation questions.", "Open clinical queue", "Medical Review", "strong", &on_navigate)}
                {workflow_action_card("QA Review", "Close sampled findings, reviewer disagreement, and feedback calibration.", "Open QA queue", "QA Review", "warning", &on_navigate)}
            </div>
        </section>
    }
}

pub fn discovery_review_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Rule & Model Discovery Review"}</h2>
                    <p>{"Use this as the single business entry for ML-discovered rule candidates and provider-trained model versions. Operators compare evidence, run backtests, inspect shadow gates, then accept or reject before anything can affect routing."}</p>
                </div>
                <span class="status-pill">{"ML governance control"}</span>
            </div>

            <section class="panel result-stack">
                <div class="section-header">
                    <div>
                        <h3>{"Candidate Review Path"}</h3>
                        <p>{"Every candidate must show source, backtest or evaluation evidence, shadow comparison, review-capacity impact, human decision, and rollback path before it can affect routing."}</p>
                    </div>
                    <span class="status-token strong">{"human approval required"}</span>
                </div>
                <div class="inbox-pipeline release-decision-flow">
                    {pipeline_step("Candidate", "ML discovery / provider model", "done")}
                    {pipeline_step("Evidence", "backtest + eval refs", "warning")}
                    {pipeline_step("Shadow", "compare against current routing", "pending")}
                    {pipeline_step("Review", "accept / reject", "pending")}
                    {pipeline_step("Release", "limited / active / rollback", "pending")}
                </div>
            </section>

            <section class="panel result-stack">
                <div class="section-header">
                    <div>
                        <h3>{"What Operators Decide Here"}</h3>
                        <p>{"Business users do not tune raw features or train models here. They accept or reject explainable candidates based on backtest, shadow, and governance evidence."}</p>
                    </div>
                    <span class="status-token neutral">{"release governance only"}</span>
                </div>
                <div class="summary-grid">
                    <div><span>{"ML rule intake"}</span><strong>{"Model-discovered rules from explanations, offline mining, or QA feedback"}</strong></div>
                    <div><span>{"Model intake"}</span><strong>{"Provider-trained model versions with dataset, split, metric, drift, and artifact evidence"}</strong></div>
                    <div><span>{"Decision"}</span><strong>{"Reject weak explanations, keep in shadow, accept for review, approve limited rollout, activate, or rollback"}</strong></div>
                    <div><span>{"Not here"}</span><strong>{"No ad hoc model training, no raw feature engineering, no autonomous denial"}</strong></div>
                </div>
            </section>

            <div class="workflow-card-grid">
                {workflow_action_card("ML Rule Review Queue", "Rules discovered from model explanations, offline mining, or case feedback must pass backtest, shadow review, and human accept/reject before entering the governed rule library.", "Review discovered rules", "Rules", "strong", &on_navigate)}
                {workflow_action_card("Provider Model Queue", "Provider training output arrives as candidate versions. Compare holdout, out-of-time, shadow, drift, and review-capacity metrics before activation.", "Review model evidence", "Provider Model Intake", "warning", &on_navigate)}
                {workflow_action_card("Routing Impact", "Check whether an approved release affects pre-payment, post-payment, manual review, pending evidence, QA sample, or straight-through routing.", "Check impact", "Routing Policies", "neutral", &on_navigate)}
                {workflow_action_card("Evidence Package", "Inspect dataset, feature-set, split, schema, and evaluation lineage that supports the release decision.", "Validate evidence", "Data Sources", "success", &on_navigate)}
                {workflow_action_card("Release History", "Audit approvals, activation, rollback, API call records, and agent/routing boundaries after release.", "Open governance", "Governance", "strong", &on_navigate)}
            </div>
        </section>
    }
}
