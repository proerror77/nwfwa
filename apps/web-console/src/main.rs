use gloo_net::http::Request;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

const API_KEY_DEFAULT: &str = "dev-secret";

const MODULES: &[&str] = &[
    "Claim Inbox",
    "Dashboard",
    "Runtime Scoring",
    "Rules",
    "Models",
    "Routing Policies",
    "Data Sources",
    "Factor Factory",
    "Leads & Cases",
    "Member Profile",
    "Provider Risk",
    "Medical Review",
    "Audit Sampling",
    "Knowledge Base",
    "Agent Investigator",
    "QA Review",
    "Governance",
];

const CONTRACT_PANELS: &[&str] = &[
    "Management Dashboard",
    "Rule Promotion Gates",
    "Discovery Mode",
    "Candidate Source",
    "Threshold Integrity",
    "Model Governance",
    "Deployment Boundary",
    "Profile Evidence",
    "Candidate Governance",
    "promotion_review_ready",
    "Factor Cards",
    "AUC Gain",
    "Field Governance",
    "Leakage Candidates",
    "SLA Breached",
    "QA Queue",
    "Canonical Evidence",
    "Calibration Signal",
    "Promotion Gate Governance",
    "API Call Records",
    "Guardrail Boundary",
    "Human Gate",
    "Graph Risk",
    "Clinical Signals",
    "Evidence Status",
    "Layer Coverage",
    "Knowledge Base",
    "Graph Evidence Status",
    "Confirmed Evidence",
    "Source Trace",
    "Lineage",
    "Audit Coverage",
    "Canonical Trace Coverage",
    "Canonical Trace",
    "Canonical Trace Only",
    "Input Mode",
];

const SAMPLE_INBOX_PAYLOAD: &str = r#"{
  "systemCode": "AiClaim Core",
  "transDate": "2026-05-27 21:22:31",
  "transNo": "f8d0e88391ac4685929d0ca1cb411e7a",
  "reportCase": {
    "reportNo": "SAAS0300040388200349",
    "accidentDate": 1766678400000,
    "claimReceiveDate": 1779811200000,
    "accidentReason": "outpatient",
    "calculateRisk": "N",
    "accidentPerson": {
      "insuredName": "LEE, Peter",
      "insuredNo": "D209475(0)",
      "certNo": "D209475(0)",
      "certType": "I",
      "gender": "M",
      "birthday": 1094313600000
    },
    "medicalRecordInfoList": [
      {
        "id": 425840008,
        "hospitalName": "Nanjing Tongren Hospital",
        "departmentName": "Dental",
        "diagnosisName": "Periodontitis",
        "medicalType": "outpatient",
        "medicalRecordType": "13",
        "visitDate": 1766678400000,
        "patientName": "",
        "medicalRecordInformation": "periodontal cleaning /n prescription"
      }
    ],
    "policyList": [
      {
        "policyNo": "PNSR039",
        "policyType": "2",
        "insuredName": "LEE, Peter",
        "validateDate": 1514822400000,
        "expireDate": 4070966400000,
        "productList": [
          {
            "productCode": "YBYL",
            "productName": "Medical Benefit",
            "validateDate": 1735747200000,
            "expireDate": 1767283200000,
            "claimLiabilityList": [
              {
                "liabCode": "YBYL02",
                "liabName": "Outpatient Medical",
                "validateDate": 1735747200000,
                "expireDate": 1767283200000
              }
            ]
          }
        ],
        "invoiceList": [
          {
            "invoiceNo": "1111111111",
            "feeAmount": 397.06,
            "startDate": 1766678400000,
            "endDate": 1766678400000,
            "hospitalCode": "HSP-001",
            "hospitalName": "Nanjing Tongren Hospital",
            "hospitalClass": "Level III",
            "hospitalProperty": "02",
            "hospitalCityName": "Nanjing",
            "hospitalProvinceName": "Jiangsu",
            "isHospitalInstitution": true,
            "primaryCare": true,
            "redFlag": "N",
            "medicalType": "outpatient",
            "departmentName": "Dental",
            "claimNature": "1",
            "billType": "socialSecurityBill",
            "documentType": "original",
            "socialInsuranceType": "2",
            "medicareAmount": 133.99,
            "selfPayAmount": 108.82,
            "ownExpenseAmount": 0,
            "otherAmount": 0,
            "accidentPersonName": "Wang",
            "diagnosisList": [
              {
                "detailCode": "K05.300",
                "detailName": "Chronic periodontitis",
                "icd": "K05.3",
                "name": "Chronic periodontitis",
                "primary": true
              }
            ],
            "feeList": [
              {
                "feeCategory": "westernMedicineFee",
                "medicareAmount": 21.55,
                "feeAmount": 51.51,
                "otherAmount": 0,
                "feeDetailList": [
                  {
                    "name": "Diclofenac diethylamine emulgel",
                    "amount": 51.51,
                    "selfPayAmount": 5.15,
                    "ownExpenseAmount": 0,
                    "medicalCategory": "1",
                    "medicareProrated": "10.00"
                  }
                ]
              }
            ]
          }
        ]
      }
    ]
  }
}"#;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct InboxNormalizeResponse {
    run_id: String,
    audit_id: String,
    external_message_id: Option<String>,
    idempotency_key: Option<String>,
    mapping_version: String,
    validation_result: String,
    scoring_ready: bool,
    raw_payload_ref: Option<String>,
    validation_errors: Vec<InboxValidationError>,
    canonical_claim_context: Value,
    data_quality_signals: Vec<String>,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct InboxValidationError {
    field_path: String,
    severity: String,
    remediation: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ScoreResponse {
    claim_id: String,
    risk_score: Value,
    recommended_action: Option<String>,
    audit_id: Option<String>,
    evidence_refs: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
struct CorrectionHint {
    field_path: String,
    severity: String,
    blocks_scoring: bool,
    next_action: String,
}

#[derive(Clone, Debug, PartialEq)]
enum ApiState<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

#[function_component(App)]
fn app() -> Html {
    let active = use_state(|| "Claim Inbox".to_string());

    html! {
        <div class="app">
            <aside>
                <h1>{"FWA Studio"}</h1>
                {for MODULES.iter().map(|module| {
                    let active = active.clone();
                    let module_name = (*module).to_string();
                    let is_active = *active == module_name;
                    html! {
                        <button
                            class={classes!(is_active.then_some("active"))}
                            onclick={Callback::from(move |_| active.set(module_name.clone()))}
                        >
                            {module}
                        </button>
                    }
                })}
            </aside>
            <main>
                if *active == "Claim Inbox" {
                    <ClaimInboxPage />
                } else {
                    <ModuleStatusPage title={(*active).clone()} />
                }
            </main>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ModuleStatusProps {
    title: String,
}

#[function_component(ModuleStatusPage)]
fn module_status_page(props: &ModuleStatusProps) -> Html {
    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{&props.title}</h2>
                    <p>{"This module remains part of the operations contract while the web console migrates to Yew."}</p>
                </div>
                <span class="status-pill">{"Yew shell"}</span>
            </div>
            <div class="panel">
                <h3>{"Migration Contract"}</h3>
                <p>{"Existing API, audit, QA, model, rule, and governance contracts stay in place. Claim Inbox is the first Yew-native operator workflow."}</p>
                <div class="tag-grid">
                    {for CONTRACT_PANELS.iter().map(|panel| html! { <span>{panel}</span> })}
                </div>
            </div>
        </section>
    }
}

#[function_component(ClaimInboxPage)]
fn claim_inbox_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let raw_payload = use_state(|| SAMPLE_INBOX_PAYLOAD.to_string());
    let overlay_payload = use_state(|| "{}".to_string());
    let reviewer_approved = use_state(|| false);
    let normalize_state = use_state(|| ApiState::<InboxNormalizeResponse>::Idle);
    let score_state = use_state(|| ApiState::<ScoreResponse>::Idle);

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
                    "canonical_claim_context": response.canonical_claim_context,
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

    let hints = match &*normalize_state {
        ApiState::Ready(response) => correction_hints_for(response),
        _ => Vec::new(),
    };
    let can_score = matches!(&*normalize_state, ApiState::Ready(response) if response.scoring_ready || *reviewer_approved);

    html! {
        <section class="claim-inbox">
            <div class="dashboard-header">
                <div>
                    <h2>{"Claim Inbox / Correction Review"}</h2>
                    <p>{"Normalize raw customer payloads, review validation findings, apply a correction overlay, and approve the canonical context for scoring."}</p>
                </div>
                <span class="status-pill">{"Yew"}</span>
            </div>

            <div class="inbox-grid">
                <section class="panel">
                    <h3>{"Raw Intake"}</h3>
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Raw payload"}
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
                </section>

                <section class="panel">
                    <h3>{"Correction Overlay"}</h3>
                    <div class="button-row">
                        <button onclick={use_template} disabled={!matches!(&*normalize_state, ApiState::Ready(_))}>
                            {"Use suggested overlay"}
                        </button>
                    </div>
                    <label>
                        {"Overlay JSON"}
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
                    if let Err(error) = &*merged_payload {
                        <p class="error">{error}</p>
                    }
                </section>
            </div>

            <div class="action-bar">
                <button onclick={normalize.clone()} disabled={matches!(&*normalize_state, ApiState::Loading)}>
                    {if matches!(&*normalize_state, ApiState::Loading) { "Normalizing..." } else { "Normalize" }}
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
                    {"Reviewer resolved blocking findings"}
                </label>
                <button onclick={score} disabled={!can_score || matches!(&*score_state, ApiState::Loading)}>
                    {if matches!(&*score_state, ApiState::Loading) { "Scoring..." } else { "Approve for scoring" }}
                </button>
            </div>

            <div class="inbox-grid">
                <NormalizeResultView state={(*normalize_state).clone()} hints={hints} />
                <ScoreResultView state={(*score_state).clone()} />
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct NormalizeResultProps {
    state: ApiState<InboxNormalizeResponse>,
    hints: Vec<CorrectionHint>,
}

#[function_component(NormalizeResultView)]
fn normalize_result_view(props: &NormalizeResultProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Validation Findings"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Run normalize to inspect validation, source refs, and correction hints."}</p> },
                ApiState::Loading => html! { <p>{"Normalizing inbox payload..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Validation"}</span><strong>{&response.validation_result}</strong></div>
                            <div><span>{"Scoring Ready"}</span><strong>{if response.scoring_ready { "yes" } else { "no" }}</strong></div>
                            <div><span>{"Mapping"}</span><strong>{&response.mapping_version}</strong></div>
                        </div>
                        <dl class="result-grid">
                            <div><dt>{"Run ID"}</dt><dd>{&response.run_id}</dd></div>
                            <div><dt>{"Audit ID"}</dt><dd>{&response.audit_id}</dd></div>
                            <div><dt>{"External Message"}</dt><dd>{response.external_message_id.as_deref().unwrap_or("missing")}</dd></div>
                            <div><dt>{"Raw Payload Ref"}</dt><dd>{response.raw_payload_ref.as_deref().unwrap_or("pending")}</dd></div>
                        </dl>
                        <h4>{"Correction Hints"}</h4>
                        if props.hints.is_empty() {
                            <p class="empty">{"No correction hints returned."}</p>
                        } else {
                            <div class="table-list">
                                {for props.hints.iter().map(|hint| html! {
                                    <div class="finding-row">
                                        <strong>{&hint.field_path}</strong>
                                        <span class={classes!("severity", hint.severity.clone())}>{&hint.severity}</span>
                                        <p>{&hint.next_action}</p>
                                        <small>{if hint.blocks_scoring { "blocks direct scoring" } else { "review signal" }}</small>
                                    </div>
                                })}
                            </div>
                        }
                        <h4>{"Canonical Context Preview"}</h4>
                        <pre>{pretty_json(&response.canonical_claim_context)}</pre>
                    </>
                },
            }}
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ScoreResultProps {
    state: ApiState<ScoreResponse>,
}

#[function_component(ScoreResultView)]
fn score_result_view(props: &ScoreResultProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Scoring Release"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Approve the normalized canonical context to score it through the existing risk engine."}</p> },
                ApiState::Loading => html! { <p>{"Scoring canonical context..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                            <div><span>{"Risk Score"}</span><strong>{display_value(&response.risk_score)}</strong></div>
                            <div><span>{"Action"}</span><strong>{response.recommended_action.as_deref().unwrap_or("review")}</strong></div>
                        </div>
                        <dl class="result-grid">
                            <div><dt>{"Audit ID"}</dt><dd>{response.audit_id.as_deref().unwrap_or("pending")}</dd></div>
                            <div><dt>{"Evidence Refs"}</dt><dd>{response.evidence_refs.clone().unwrap_or_default().join(", ")}</dd></div>
                        </dl>
                    </>
                },
            }}
        </section>
    }
}

async fn normalize_claim(
    payload: Value,
    api_key: String,
) -> Result<InboxNormalizeResponse, String> {
    request_json("/api/v1/inbox/claims/normalize", api_key, payload).await
}

async fn score_canonical_claim(payload: Value, api_key: String) -> Result<ScoreResponse, String> {
    request_json("/api/v1/claims/score", api_key, payload).await
}

async fn request_json<T>(path: &str, api_key: String, payload: Value) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let request = Request::post(path)
        .header("content-type", "application/json")
        .header("x-api-key", &api_key)
        .body(payload.to_string())
        .map_err(|error| error.to_string())?;
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let body: Value = response.json().await.map_err(|error| error.to_string())?;
    if !(200..300).contains(&status) {
        return Err(body
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}: {}", pretty_json(&body))));
    }
    serde_json::from_value(body).map_err(|error| error.to_string())
}

fn merge_payload_text(raw_payload: &str, overlay_payload: &str) -> Result<Value, String> {
    let mut payload = serde_json::from_str::<Value>(raw_payload)
        .map_err(|error| format!("raw payload JSON is invalid: {error}"))?;
    let overlay = serde_json::from_str::<Value>(overlay_payload)
        .map_err(|error| format!("correction overlay JSON is invalid: {error}"))?;
    merge_overlay(&mut payload, &overlay);
    Ok(payload)
}

fn merge_overlay(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(key) {
                    Some(base_value) => merge_overlay(base_value, value),
                    None => {
                        base.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (Value::Array(base), Value::Array(overlay)) => {
            for (index, value) in overlay.iter().enumerate() {
                if let Some(base_value) = base.get_mut(index) {
                    merge_overlay(base_value, value);
                } else {
                    base.push(value.clone());
                }
            }
        }
        (base, overlay) => *base = overlay.clone(),
    }
}

fn correction_hints_for(response: &InboxNormalizeResponse) -> Vec<CorrectionHint> {
    if response.scoring_ready {
        return Vec::new();
    }
    response
        .validation_errors
        .iter()
        .map(|error| CorrectionHint {
            field_path: error.field_path.clone(),
            severity: error.severity.clone(),
            blocks_scoring: blocks_direct_scoring(&error.field_path, &error.severity),
            next_action: next_action_for_validation_error(error),
        })
        .collect()
}

fn blocks_direct_scoring(field_path: &str, severity: &str) -> bool {
    if severity != "warning" || !field_path.starts_with("reportCase.policyList[") {
        return false;
    }
    if field_path.contains(".invoiceList[") {
        return false;
    }
    let field = field_path.rsplit('.').next().unwrap_or_default();
    if field_path.contains(".productList[") {
        matches!(field, "validateDate" | "claimValidateDate" | "expireDate")
    } else {
        matches!(field, "coverageLimit" | "validateDate" | "expireDate")
    }
}

fn next_action_for_validation_error(error: &InboxValidationError) -> String {
    if error.field_path == "systemCode" {
        return "use an API key/source-system config that matches the payload systemCode".into();
    }
    if error.field_path.ends_with(".coverageLimit") {
        return "map the policy or liability coverage limit before direct scoring".into();
    }
    if error.field_path.ends_with(".validateDate")
        || error.field_path.ends_with(".expireDate")
        || error.field_path.ends_with(".claimValidateDate")
    {
        return "fix or reviewer-resolve the policy/product/liability date window before scoring"
            .into();
    }
    if error.field_path == "reportCase.calculateRisk" {
        return "keep the payload in the FWA audit path unless customer config explicitly allows bypass"
            .into();
    }
    if error.remediation.is_empty() {
        "review this field before scoring".into()
    } else {
        error.remediation.clone()
    }
}

fn correction_overlay_template_for(errors: &[InboxValidationError]) -> Value {
    let mut template = json!({});
    for error in errors {
        apply_overlay_template_field(&mut template, &error.field_path);
    }
    template
}

fn apply_overlay_template_field(template: &mut Value, field_path: &str) {
    let Some(after_policy) = field_path.strip_prefix("reportCase.policyList[") else {
        return;
    };
    let Some((policy_index, rest)) = consume_index(after_policy) else {
        return;
    };

    if matches!(rest, "coverageLimit" | "validateDate" | "expireDate") {
        set_policy_field(
            template,
            policy_index,
            rest,
            placeholder_for("policy", rest),
        );
        return;
    }

    let Some(after_product) = rest.strip_prefix("productList[") else {
        return;
    };
    let Some((product_index, rest)) = consume_index(after_product) else {
        return;
    };
    if matches!(rest, "validateDate" | "expireDate" | "claimValidateDate") {
        set_product_field(
            template,
            policy_index,
            product_index,
            rest,
            placeholder_for("product", rest),
        );
        return;
    }

    let Some(after_liability) = rest.strip_prefix("claimLiabilityList[") else {
        return;
    };
    let Some((liability_index, rest)) = consume_index(after_liability) else {
        return;
    };
    if matches!(rest, "validateDate" | "expireDate" | "claimValidateDate") {
        set_liability_field(
            template,
            policy_index,
            product_index,
            liability_index,
            rest,
            placeholder_for("liability", rest),
        );
    }
}

fn consume_index(value: &str) -> Option<(usize, &str)> {
    let (index, rest) = value.split_once("].")?;
    Some((index.parse().ok()?, rest))
}

fn set_policy_field(template: &mut Value, policy_index: usize, field: &str, value: Value) {
    let policy = policy_template(template, policy_index);
    ensure_object(policy).insert(field.into(), value);
}

fn set_product_field(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    field: &str,
    value: Value,
) {
    let product = product_template(template, policy_index, product_index);
    ensure_object(product).insert(field.into(), value);
}

fn set_liability_field(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    liability_index: usize,
    field: &str,
    value: Value,
) {
    let liability = liability_template(template, policy_index, product_index, liability_index);
    ensure_object(liability).insert(field.into(), value);
}

fn policy_template(template: &mut Value, policy_index: usize) -> &mut Value {
    let report_case = ensure_object(template)
        .entry("reportCase")
        .or_insert_with(|| json!({}));
    let policies = ensure_object(report_case)
        .entry("policyList")
        .or_insert_with(|| json!([]));
    let policies = ensure_array(policies);
    while policies.len() <= policy_index {
        policies.push(json!({}));
    }
    &mut policies[policy_index]
}

fn product_template(template: &mut Value, policy_index: usize, product_index: usize) -> &mut Value {
    let policy = policy_template(template, policy_index);
    let products = ensure_object(policy)
        .entry("productList")
        .or_insert_with(|| json!([]));
    let products = ensure_array(products);
    while products.len() <= product_index {
        products.push(json!({}));
    }
    &mut products[product_index]
}

fn liability_template(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    liability_index: usize,
) -> &mut Value {
    let product = product_template(template, policy_index, product_index);
    let liabilities = ensure_object(product)
        .entry("claimLiabilityList")
        .or_insert_with(|| json!([]));
    let liabilities = ensure_array(liabilities);
    while liabilities.len() <= liability_index {
        liabilities.push(json!({}));
    }
    &mut liabilities[liability_index]
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    value
        .as_object_mut()
        .expect("value was converted to object")
}

fn ensure_array(value: &mut Value) -> &mut Vec<Value> {
    if !value.is_array() {
        *value = json!([]);
    }
    value.as_array_mut().expect("value was converted to array")
}

fn placeholder_for(scope: &str, field: &str) -> Value {
    if field == "coverageLimit" {
        return Value::String("<REQUIRED_COVERAGE_LIMIT>".into());
    }
    let mut label = String::new();
    for (index, character) in field.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            label.push('_');
        }
        label.push(character.to_ascii_uppercase());
    }
    Value::String(format!(
        "<REQUIRED_{}_{}_EPOCH_MS>",
        scope.to_ascii_uppercase(),
        label
    ))
}

fn source_system_from_context(context: &Value) -> String {
    context
        .pointer("/claim_header/source_system")
        .and_then(Value::as_str)
        .unwrap_or("AiClaim Core")
        .to_string()
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
}

fn display_value(value: &Value) -> String {
    value
        .as_f64()
        .map(|number| format!("{number:.1}"))
        .or_else(|| value.as_str().map(str::to_string))
        .unwrap_or_else(|| value.to_string())
}

fn main() {
    yew::Renderer::<App>::new().render();
}
