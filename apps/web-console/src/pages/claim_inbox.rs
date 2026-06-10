use crate::api::*;
use crate::types::*;
use crate::constants::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::case_helpers::*;
use crate::inbox_helpers::*;
use crate::payload_helpers::*;
use yew::prelude::*;
use serde_json::{json, Value};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};

#[path = "claim_inbox_view.rs"]
mod claim_inbox_view;
use claim_inbox_view::{LiveTpaDemoView, NormalizeResultView, ScoreResultView};

#[function_component(ClaimInboxPage)]
pub fn claim_inbox_page() -> Html {
    let api_key = use_api_key();
    let raw_payload = use_state(|| SAMPLE_INBOX_PAYLOAD.to_string());
    let overlay_payload = use_state(|| "{}".to_string());
    let reviewer_approved = use_state(|| false);
    let normalize_state = use_state(|| ApiState::<InboxNormalizeResponse>::Idle);
    let score_state = use_state(|| ApiState::<ScoreResponse>::Idle);
    let live_demo_state = use_state(|| ApiState::<LiveTpaDemoRun>::Idle);

    let merged_payload = use_memo(
        ((*raw_payload).clone(), (*overlay_payload).clone()),
        |(raw_payload, overlay_payload)| merge_payload_text(raw_payload, overlay_payload),
    );

    let normalize = {
        let api_key = api_key.clone();
        let merged_payload = merged_payload.clone();
        let normalize_state = normalize_state.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let normalize_state = normalize_state.clone();
            let score_state = score_state.clone();
            match &*merged_payload {
                Ok(payload) => {
                    let payload = payload.clone();
                    normalize_state.set(ApiState::Loading);
                    score_state.set(ApiState::Idle);
                    spawn_local(async move {
                        normalize_state.set(match normalize_claim(payload, api_key).await {
                            Ok(response) => ApiState::Ready(response),
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => normalize_state.set(ApiState::Failed(error.clone())),
            }
        })
    };

    let use_template = {
        let overlay_payload = overlay_payload.clone();
        let normalize_state = normalize_state.clone();
        Callback::from(move |_| {
            if let ApiState::Ready(response) = &*normalize_state {
                let template = correction_overlay_template_for(&response.validation_errors);
                overlay_payload.set(pretty_json(&template));
            }
        })
    };

    let score = {
        let api_key = api_key.clone();
        let normalize_state = normalize_state.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            if let ApiState::Ready(response) = &*normalize_state {
                let api_key = (*api_key).clone();
                let score_state = score_state.clone();
                let payload = json!({
                    "source_system": source_system_from_context(&response.canonical_claim_context),
                    "inbox_run_id": response.run_id.clone(),
                });
                score_state.set(ApiState::Loading);
                spawn_local(async move {
                    score_state.set(match score_canonical_claim(payload, api_key).await {
                        Ok(response) => ApiState::Ready(response),
                        Err(error) => ApiState::Failed(error),
                    });
                });
            }
        })
    };

    let run_live_demo = {
        let api_key = api_key.clone();
        let normalize_state = normalize_state.clone();
        let score_state = score_state.clone();
        let live_demo_state = live_demo_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let normalize_state = normalize_state.clone();
            let score_state = score_state.clone();
            let live_demo_state = live_demo_state.clone();
            normalize_state.set(ApiState::Loading);
            score_state.set(ApiState::Idle);
            live_demo_state.set(ApiState::Loading);
            spawn_local(async move {
                let result = async {
                    let before_dashboard = get_dashboard_summary(api_key.clone()).await?;
                    let payload = live_tpa_demo_payload(&before_dashboard)?;
                    let normalize_response = normalize_claim(payload, api_key.clone()).await?;
                    normalize_state.set(ApiState::Ready(normalize_response.clone()));
                    if !normalize_response.scoring_ready {
                        return Err("live demo packet did not pass intake normalization".into());
                    }
                    let score_response = score_canonical_claim(
                        json!({
                            "source_system": source_system_from_context(&normalize_response.canonical_claim_context),
                            "inbox_run_id": normalize_response.run_id.clone(),
                        }),
                        api_key.clone(),
                    )
                    .await?;
                    score_state.set(ApiState::Ready(score_response.clone()));
                    let score_run_id = score_response
                        .run_id
                        .clone()
                        .ok_or_else(|| "score response did not include a run id".to_string())?;
                    let snapshot = get_leads_cases_snapshot(api_key.clone()).await?;
                    let lead = latest_lead_for_score(
                        &snapshot,
                        &score_response.claim_id,
                        &score_run_id,
                    )
                    .ok_or_else(|| {
                        format!(
                            "no generated lead found for {} / {}",
                            score_response.claim_id, score_run_id
                        )
                    })?;
                    let triage = post_triage_lead(
                        api_key.clone(),
                        lead.lead_id.clone(),
                        json!({
                            "decision": "open_case",
                            "merge_target_lead_id": Value::Null,
                            "assignee": "demo-investigator",
                            "reviewer": "demo-reviewer",
                            "priority": "high",
                            "notes": "Live TPA demo opens a governed FWA investigation case.",
                            "evidence_refs": if lead.evidence_refs.is_empty() {
                                vec![format!("leads:{}", lead.lead_id)]
                            } else {
                                lead.evidence_refs.clone()
                            },
                        }),
                    )
                    .await?;
                    let case = triage
                        .case
                        .ok_or_else(|| "triage did not open an investigation case".to_string())?;
                    let case_update = post_case_status(
                        api_key.clone(),
                        case.case_id.clone(),
                        json!({
                            "status": "investigating",
                            "actor_id": "demo-investigator",
                            "notes": "Live TPA demo investigation started from the triaged lead.",
                            "evidence_refs": [
                                format!("investigation_cases:{}", case.case_id),
                                format!("audit:{}", triage.audit_id),
                            ],
                        }),
                    )
                    .await?;
                    let score_audit_id = score_response
                        .audit_id
                        .clone()
                        .unwrap_or_else(|| score_run_id.clone());
                    let investigation = post_investigation_result(
                        api_key.clone(),
                        json!({
                            "case_id": case.case_id,
                            "claim_id": score_response.claim_id,
                            "investigation_id": format!("INV-LIVE-{}", score_run_id),
                            "outcome": "confirmed_fwa_prevented_payment",
                            "confirmed_fwa": true,
                            "financial_impact_type": "prevented_payment",
                            "saving_amount": LIVE_TPA_DEMO_AMOUNT,
                            "currency": "CNY",
                            "notes": "Demo reviewer confirmed the pre-payment FWA intervention and prevented payment.",
                            "evidence_refs": [
                                format!("investigation_cases:{}", case_update.case.case_id),
                                format!("audit:{}", score_audit_id),
                            ],
                        }),
                    )
                    .await?;
                    let after_dashboard = get_dashboard_summary(api_key).await?;
                    Ok(LiveTpaDemoRun {
                        claim_id: score_response.claim_id,
                        claim_amount: LIVE_TPA_DEMO_AMOUNT.to_string(),
                        inbox_run_id: normalize_response.run_id,
                        score_run_id,
                        risk_score: display_value(&score_response.risk_score),
                        rag: score_response
                            .rag
                            .as_ref()
                            .map(display_value)
                            .unwrap_or_else(|| "missing".into()),
                        decision_outcome: score_response
                            .decision_outcome
                            .unwrap_or_else(|| "review".into()),
                        lead_id: lead.lead_id.clone(),
                        case_id: case_update.case.case_id,
                        case_status: case_update.case.status,
                        investigation_audit_id: investigation.audit_id,
                        prevented_before: before_dashboard.value_measurement.prevented_payment,
                        prevented_after: after_dashboard.value_measurement.prevented_payment,
                        dashboard_saving_after: after_dashboard.saving_amount,
                    })
                }
                .await;
                live_demo_state.set(match result {
                    Ok(run) => ApiState::Ready(run),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let hints = match &*normalize_state {
        ApiState::Ready(response) => correction_hints_for(response),
        _ => Vec::new(),
    };
    let can_score = matches!(&*normalize_state, ApiState::Ready(response) if response.scoring_ready || *reviewer_approved);

    html! {
        <section class="claim-inbox">
            <div class="dashboard-header">
                <div>
                    <h2>{"Intake Ops"}</h2>
                    <p>{"Review inbound TPA claim packets, resolve intake blockers, and release accepted claims into the risk and review queue."}</p>
                </div>
                <span class="status-pill">{"Intake Ops"}</span>
            </div>

            <div class="inbox-grid">
                <section class="panel">
                    <h3>{"Inbound Claim Packet"}</h3>
                    <p class="empty">{"Use the configured intake channel to check whether the claim packet is complete enough for downstream review."}</p>
                    <div class="summary-grid">
                        <div><span>{"Source"}</span><strong>{"TPA intake"}</strong></div>
                        <div><span>{"Packet"}</span><strong>{"sample loaded"}</strong></div>
                        <div><span>{"Next step"}</span><strong>{"check intake packet"}</strong></div>
                    </div>
                    <details>
                        <summary>{"Technical payload editor"}</summary>
                        <label>
                            {"Payload JSON"}
                            <textarea
                                class="payload-editor"
                                value={(*raw_payload).clone()}
                                oninput={{
                                    let raw_payload = raw_payload.clone();
                                    Callback::from(move |event: InputEvent| {
                                        raw_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                    </details>
                </section>

                <section class="panel">
                    <h3>{"Correction Worklist"}</h3>
                    <p class="empty">{"After intake checks run, prepare only the missing or reviewer-approved fixes needed for queue release."}</p>
                    <div class="button-row">
                        <button onclick={use_template} disabled={!matches!(&*normalize_state, ApiState::Ready(_))}>
                            {"Prepare correction draft"}
                        </button>
                    </div>
                    <details>
                        <summary>{"Technical correction editor"}</summary>
                        <label>
                            {"Correction JSON"}
                            <textarea
                                class="payload-editor"
                                value={(*overlay_payload).clone()}
                                oninput={{
                                    let overlay_payload = overlay_payload.clone();
                                    Callback::from(move |event: InputEvent| {
                                        overlay_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                    </details>
                    if let Err(error) = &*merged_payload {
                        <p class="error">{error}</p>
                    }
                </section>
            </div>

            <section class="panel result-stack live-demo-panel">
                <div class="section-header">
                    <div>
                        <h3>{"Live TPA Demo Run"}</h3>
                        <p>{"Show one raw TPA packet becoming a scored lead, investigation case, human writeback, and value proof without switching scripts mid-demo."}</p>
                    </div>
                    <span class="status-token strong">{"TPA packet -> risk queue -> case -> value proof"}</span>
                </div>
                <div class="inbox-pipeline live-demo-flow">
                    {pipeline_step("Receive", "raw TPA packet", "done")}
                    {pipeline_step("Normalize", "canonical claim", if matches!(&*normalize_state, ApiState::Ready(_)) { "done" } else { "pending" })}
                    {pipeline_step("Score", "risk + routing", if matches!(&*score_state, ApiState::Ready(_)) { "done" } else { "pending" })}
                    {pipeline_step("Investigate", "lead + case", if matches!(&*live_demo_state, ApiState::Ready(_)) { "done" } else { "pending" })}
                    {pipeline_step("Prove Value", "prevented payment", if matches!(&*live_demo_state, ApiState::Ready(_)) { "done" } else { "pending" })}
                </div>
                <div class="button-row">
                    <button onclick={run_live_demo} disabled={matches!(&*live_demo_state, ApiState::Loading)}>
                        {if matches!(&*live_demo_state, ApiState::Loading) { "Running live demo..." } else { "Run full TPA demo" }}
                    </button>
                </div>
                <LiveTpaDemoView state={(*live_demo_state).clone()} />
            </section>

            <div class="action-bar">
                <button onclick={normalize.clone()} disabled={matches!(&*normalize_state, ApiState::Loading)}>
                    {if matches!(&*normalize_state, ApiState::Loading) { "Checking..." } else { "Check intake packet" }}
                </button>
                <label class="checkbox-row">
                    <input
                        type="checkbox"
                        checked={*reviewer_approved}
                        onchange={{
                            let reviewer_approved = reviewer_approved.clone();
                            Callback::from(move |event: Event| {
                                reviewer_approved.set(event.target_unchecked_into::<HtmlInputElement>().checked());
                            })
                        }}
                    />
                    {"Reviewer confirms required intake fixes"}
                </label>
                <button onclick={score} disabled={!can_score || matches!(&*score_state, ApiState::Loading)}>
                    {if matches!(&*score_state, ApiState::Loading) { "Releasing..." } else { "Release accepted claim" }}
                </button>
            </div>

            <div class="inbox-grid">
                <NormalizeResultView state={(*normalize_state).clone()} hints={hints} />
                <ScoreResultView state={(*score_state).clone()} />
            </div>
        </section>
    }
}
