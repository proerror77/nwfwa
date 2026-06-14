use crate::case_helpers::*;
use crate::formatting::*;
use crate::inbox_helpers::*;
use crate::state::ApiState;
use crate::types::*;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(super) struct NormalizeResultProps {
    pub(super) state: ApiState<InboxNormalizeResponse>,
    pub(super) hints: Vec<CorrectionHint>,
}

#[function_component(NormalizeResultView)]
pub(super) fn normalize_result_view(props: &NormalizeResultProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Intake Findings"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Check the intake packet to see blockers, warnings, and required fixes."}</p> },
                ApiState::Loading => html! { <p>{"Checking intake packet..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero compact-metrics">
                            <div><span>{"Validation"}</span><strong>{business_label(&response.validation_result)}</strong></div>
                            <div><span>{"Queue Ready"}</span><strong>{if response.scoring_ready { "Ready" } else { "Needs review" }}</strong></div>
                            <div><span>{"Mapping"}</span><strong>{&response.mapping_version}</strong></div>
                        </div>
                        {inbox_pipeline_visual(response)}
                        {validation_findings_visual(response, &props.hints)}
                        <details>
                            <summary>{"Audit trace"}</summary>
                            <dl class="result-grid">
                                <div><dt>{"Run ID"}</dt><dd>{&response.run_id}</dd></div>
                                <div><dt>{"Audit ID"}</dt><dd>{&response.audit_id}</dd></div>
                                <div><dt>{"External Message"}</dt><dd>{response.external_message_id.as_deref().unwrap_or("missing")}</dd></div>
                                <div><dt>{"Payload Ref"}</dt><dd>{response.raw_payload_ref.as_deref().unwrap_or("pending")}</dd></div>
                            </dl>
                        </details>
                        <h4>{"Required Fixes"}</h4>
                        if props.hints.is_empty() {
                            <p class="empty">{"No correction hints returned."}</p>
                        } else {
                            <div class="table-list finding-list">
                                {for props.hints.iter().map(|hint| html! {
                                    <div class="finding-row">
                                        <strong>{&hint.field_path}</strong>
                                        <span class={classes!("severity", hint.severity.clone())}>{business_label(&hint.severity)}</span>
                                        <p>{&hint.next_action}</p>
                                        <small>{if hint.blocks_scoring { "blocks queue release" } else { "review signal" }}</small>
                                    </div>
                                })}
                            </div>
                        }
                        <details>
                            <summary>{"Canonical context preview"}</summary>
                            <pre>{pretty_json(&response.canonical_claim_context)}</pre>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

#[derive(Properties, PartialEq)]
pub(super) struct ScoreResultProps {
    pub(super) state: ApiState<ScoreResponse>,
}

#[function_component(ScoreResultView)]
pub(super) fn score_result_view(props: &ScoreResultProps) -> Html {
    html! {
        <section class="panel result-stack queue-handoff-panel">
            <h3>{"Queue Handoff"}</h3>
            {match &props.state {
                ApiState::Idle => html! {
                    <div class="handoff-status pending">
                        <span>{"Not released"}</span>
                        <strong>{"Waiting for intake check"}</strong>
                        <small>{"Accepted claims enter Leads & Cases or review queues after release."}</small>
                    </div>
                },
                ApiState::Loading => html! {
                    <div class="handoff-status pending">
                        <span>{"Release in progress"}</span>
                        <strong>{"Creating queue handoff"}</strong>
                        <small>{"The claim is being checked by the risk service before downstream routing."}</small>
                    </div>
                },
                ApiState::Failed(error) => html! {
                    <>
                        <div class="handoff-status blocked">
                            <span>{"Not released"}</span>
                            <strong>{release_blocker_title(error)}</strong>
                            <small>{release_blocker_next_step(error)}</small>
                        </div>
                        <details>
                            <summary>{"Diagnostic detail"}</summary>
                            <p class="empty">{error}</p>
                        </details>
                    </>
                },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="handoff-status done">
                            <span>{"Released"}</span>
                            <strong>{"Claim entered downstream queue"}</strong>
                            <small>{"Reviewers continue the case from Leads & Cases or Review Workbench."}</small>
                        </div>
                        <div class="score-hero compact-metrics">
                            <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                            <div><span>{"Risk Score"}</span><strong>{display_value(&response.risk_score)}</strong></div>
                            <div><span>{"Queue Route"}</span><strong>{response.recommended_action.as_deref().map(business_label).unwrap_or_else(|| "Manual review".into())}</strong></div>
                        </div>
                        <details>
                            <summary>{"Release trace"}</summary>
                            <dl class="result-grid">
                                <div><dt>{"Audit ID"}</dt><dd>{response.audit_id.as_deref().unwrap_or("pending")}</dd></div>
                                <div><dt>{"Evidence Refs"}</dt><dd>{response.evidence_refs.as_ref().map(|refs| value_refs_label(refs)).unwrap_or_else(|| "none".into())}</dd></div>
                            </dl>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

#[derive(Properties, PartialEq)]
pub(super) struct LiveTpaDemoProps {
    pub(super) state: ApiState<LiveTpaDemoRun>,
}

#[function_component(LiveTpaDemoView)]
pub(super) fn live_tpa_demo_view(props: &LiveTpaDemoProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! {
                    <p class="empty">{"Run the full TPA demo when you want the audience to see the system move from inbound packet to prevented-payment value proof."}</p>
                },
                ApiState::Loading => html! {
                    <div class="handoff-status pending">
                        <span>{"Live demo running"}</span>
                        <strong>{"Normalizing, scoring, opening case, and writing back outcome"}</strong>
                        <small>{"The UI is calling the same APIs that the external TPA demo script calls."}</small>
                    </div>
                },
                ApiState::Failed(error) => html! {
                    <div class="handoff-status blocked">
                        <span>{"Live demo stopped"}</span>
                        <strong>{"Fix the runtime before presenting"}</strong>
                        <small>{error}</small>
                    </div>
                },
                ApiState::Ready(run) => html! {
                    <>
                        <div class="handoff-status done">
                            <span>{"Live demo complete"}</span>
                            <strong>{format!("{} prevented payment recorded", run.claim_amount)}</strong>
                            <small>{"The claim is now visible in Leads & Cases and the value proof dashboard reflects the writeback."}</small>
                        </div>
                        <div class="score-hero compact-metrics">
                            <div><span>{"Claim"}</span><strong>{&run.claim_id}</strong></div>
                            <div><span>{"Risk"}</span><strong>{format!("{} / {}", run.risk_score, rag_label(&run.rag))}</strong></div>
                            <div><span>{"Decision"}</span><strong>{business_label(&run.decision_outcome)}</strong></div>
                        </div>
                        <div class="summary-grid">
                            <div><span>{"Inbox run"}</span><strong>{&run.inbox_run_id}</strong></div>
                            <div><span>{"Score run"}</span><strong>{&run.score_run_id}</strong></div>
                            <div><span>{"Lead"}</span><strong>{&run.lead_id}</strong></div>
                            <div><span>{"Case"}</span><strong>{format!("{} / {}", run.case_id, case_stage_label(&run.case_status))}</strong></div>
                            <div><span>{"Investigation audit"}</span><strong>{&run.investigation_audit_id}</strong></div>
                            <div><span>{"Dashboard value"}</span><strong>{format!("{} -> {}", run.prevented_before, run.prevented_after)}</strong></div>
                        </div>
                        <small>{format!("confirmed dashboard saving amount: {}", run.dashboard_saving_after)}</small>
                    </>
                },
            }}
        </>
    }
}

fn release_blocker_title(error: &str) -> &'static str {
    if error.contains("coverage_limit") || error.contains("coverage limit") {
        "Coverage limit needs correction"
    } else if error.contains("claim_amount") || error.contains("claim amount") {
        "Claim amount needs confirmation"
    } else {
        "Claim packet is not ready"
    }
}

fn release_blocker_next_step(error: &str) -> &'static str {
    if error.contains("coverage_limit") || error.contains("coverage limit") {
        "Update the policy or liability coverage limit, then check the intake packet again."
    } else if error.contains("claim_amount") || error.contains("claim amount") {
        "Confirm the payable claim amount from invoice totals before release."
    } else {
        "Resolve the intake findings on the left before releasing this claim."
    }
}
