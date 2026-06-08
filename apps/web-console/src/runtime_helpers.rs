use crate::{payload_keys_label, risk_node, RuntimeModelScore, RuntimeRequiredEvidence, ScoreResponse};
use serde_json::{json, Value};
use yew::prelude::*;

pub(crate) fn required_evidence_label(items: &[RuntimeRequiredEvidence]) -> String {
    items
        .iter()
        .map(|item| {
            let mut label = item.evidence_type.clone();
            if let Some(request_type) = item.evidence_request_type.as_deref() {
                label = format!("{label} / {request_type}");
            }
            if item.blocking {
                label.push_str(" / blocking");
            }
            if let Some(authority_ref) = item.policy_authority_ref.as_deref() {
                label = format!("{label} / {authority_ref}");
            }
            if let Some(exception_check) = item.exception_check.as_deref() {
                label = format!("{label} / {exception_check}");
            }
            label
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn runtime_score_breakdown(response: &ScoreResponse) -> Html {
    if let Some(scores) = &response.scores {
        html! {
            <div class="risk-flow signal-score-grid">
                {risk_node("Peer", "Deviation", &scores.peer_deviation_score.to_string(), "claim amount / stay / frequency")}
                {risk_node("Rules", "Controls", &scores.rule_score.to_string(), "deterministic policy checks")}
                {risk_node("Anomaly", "Pattern", &scores.anomaly_score.to_string(), "rare utilization behavior")}
                {risk_node("Model", "Classifier", &scores.ml_score.to_string(), "trained runtime score")}
                {risk_node("Clinical", "Necessity", &scores.medical_reasonableness_score.to_string(), "medical reasonableness")}
                {risk_node("Provider", "Network", &scores.provider_network_score.to_string(), "relationship and graph risk")}
                {risk_node("Knowledge", "Similar cases", &scores.similar_case_score.to_string(), "confirmed case memory")}
                {risk_node("Route", "Policy score", &scores.final_score.to_string(), "downstream human queue")}
            </div>
        }
    } else {
        html! { <p class="empty">{"No score breakdown returned."}</p> }
    }
}

pub(crate) fn runtime_model_output(model_score: Option<&RuntimeModelScore>) -> Html {
    if let Some(model) = model_score {
        html! {
            <div class="result-stack">
                <div class="summary-grid">
                    <div><span>{"Model"}</span><strong>{format!("{} {}", model.model_key, model.model_version)}</strong></div>
                    <div><span>{"Runtime"}</span><strong>{format!("{} / {}", model.runtime_kind, model.execution_provider)}</strong></div>
                    <div><span>{"Score"}</span><strong>{model.score}</strong></div>
                    <div><span>{"Label"}</span><strong>{&model.label}</strong></div>
                    <div><span>{"Latency"}</span><strong>{format!("{} ms", model.latency_ms)}</strong></div>
                    <div><span>{"Metadata"}</span><strong>{payload_keys_label(&model.metadata)}</strong></div>
                </div>
                if model.explanations.is_empty() {
                    <p class="empty">{"No model explanations returned."}</p>
                } else {
                    <div class="factor-card-grid">
                        {for model.explanations.iter().map(|explanation| html! {
                            <div class="metric-row">
                                <span>{&explanation.feature}</span>
                                <strong>{format!("{} {:.2}", explanation.direction, explanation.contribution)}</strong>
                                <small>{&explanation.reason}</small>
                            </div>
                        })}
                    </div>
                }
            </div>
        }
    } else {
        html! { <p class="empty">{"No model score returned."}</p> }
    }
}

pub(crate) fn runtime_full_payload_template() -> Value {
    json!({
        "source_system": "tpa-demo",
        "review_mode": "pre_payment",
        "claim": {
            "external_claim_id": "CLM-WEB-RUNTIME",
            "claim_amount": "18900",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10",
            "items": [
                {
                    "item_code": "IMG-001",
                    "item_type": "procedure",
                    "description": "High cost imaging",
                    "quantity": 1,
                    "unit_amount": "18900",
                    "total_amount": "18900",
                    "currency": "CNY"
                }
            ],
            "member": {
                "external_member_id": "MBR-WEB-RUNTIME",
                "dob": "1985-03-14",
                "gender": "F"
            },
            "policy": {
                "external_policy_id": "POL-WEB-RUNTIME",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "20000",
                "currency": "CNY"
            },
            "provider": {
                "external_provider_id": "PRV-WEB-RUNTIME",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "Shanghai",
                "risk_tier": "High"
            },
            "documents": [
                {
                    "external_document_id": "DOC-WEB-RUNTIME",
                    "document_type": "medical_record",
                    "linked_item_codes": ["IMG-001"]
                }
            ],
            "provider_profile": {
                "specialty": "general",
                "network_status": "in_network",
                "windows": [
                    {
                        "window_days": 30,
                        "claim_count": 40,
                        "total_claim_amount": "480000",
                        "high_cost_item_ratio": 0.74,
                        "diagnosis_procedure_mismatch_rate": 0.46,
                        "peer_amount_percentile": 96,
                        "peer_frequency_percentile": 93,
                        "review_failure_count": 8,
                        "confirmed_fwa_count": 3,
                        "false_positive_count": 1
                    }
                ]
            },
            "provider_relationships": {
                "high_risk_neighbor_ratio": 0.42,
                "provider_patient_overlap_score": 0.72,
                "referral_concentration_score": 0.66,
                "connected_confirmed_fwa_count": 4,
                "network_component_risk_score": 84,
                "evidence_refs": ["provider_graph:PRV-WEB-RUNTIME"]
            }
        }
    })
}
