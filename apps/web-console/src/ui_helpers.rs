use crate::state::Language;
use crate::{map_counts_label, AgentApprovalView};
use std::collections::BTreeMap;
use web_sys::HtmlInputElement;
use yew::prelude::*;

pub(crate) fn timeline_item(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("timeline-item", status_tone(tone))}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

pub(crate) fn case_action(label: &str, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("case-action", tone.to_string())}>
            <strong>{label}</strong>
            <span>{caption}</span>
        </div>
    }
}

pub(crate) fn evidence_chain(refs: &[String], language: Language) -> Html {
    let groups = [
        ("claim", ref_label(language, "Claim", "理赔")),
        ("audit", ref_label(language, "Audit", "审计")),
        ("rule", ref_label(language, "Rule", "规则")),
        ("model", ref_label(language, "Model", "模型")),
        ("document", ref_label(language, "Document", "文件")),
        ("similar", ref_label(language, "Similar case", "相似案例")),
        ("other", ref_label(language, "Other", "其他")),
    ];
    html! {
        <section class="evidence-chain">
            <div class="section-header">
                <div>
                    <h3>{ref_label(language, "Evidence chain", "证据链")}</h3>
                    <p>{ref_label(language, "Grouped refs used to explain this case and support final writeback.", "按类型整理本案解释和最终结论需要引用的证据。")}</p>
                </div>
                <span class="status-token strong">{evidence_refs_count(refs.len(), language)}</span>
            </div>
            { if refs.is_empty() {
                html! { <p class="empty">{ref_label(language, "No evidence refs attached.", "暂无证据引用。")}</p> }
            } else {
                html! {
                    <div class="evidence-chain-grid">
                        { for groups.into_iter().map(|(kind, label)| evidence_chain_group(kind, label, refs)) }
                    </div>
                }
            }}
        </section>
    }
}

fn evidence_chain_group(kind: &'static str, label: &'static str, refs: &[String]) -> Html {
    let items = refs
        .iter()
        .filter(|reference| evidence_ref_kind(reference) == kind)
        .collect::<Vec<_>>();
    if items.is_empty() {
        return html! {};
    }
    html! {
        <div class="evidence-chain-group">
            <span>{label}</span>
            { for items.into_iter().take(4).map(|reference| html! { <code>{reference}</code> }) }
        </div>
    }
}

fn evidence_ref_kind(reference: &str) -> &'static str {
    let reference = reference.to_ascii_lowercase();
    if reference.starts_with("claim") || reference.starts_with("investigation_cases:") {
        "claim"
    } else if reference.starts_with("audit") {
        "audit"
    } else if reference.starts_with("rule") {
        "rule"
    } else if reference.starts_with("model") {
        "model"
    } else if reference.contains("document")
        || reference.contains("medical_record")
        || reference.contains("invoice")
    {
        "document"
    } else if reference.starts_with("knowledge_cases:") || reference.starts_with("similar") {
        "similar"
    } else {
        "other"
    }
}

fn ref_label(language: Language, en: &'static str, zh: &'static str) -> &'static str {
    match language {
        Language::En => en,
        Language::Zh => zh,
    }
}

fn evidence_refs_count(count: usize, language: Language) -> String {
    match language {
        Language::En => format!("{count} refs"),
        Language::Zh => format!("{count} 条引用"),
    }
}

pub(crate) fn scaled_width(value: u32, max_value: u32) -> String {
    let width = if max_value == 0 {
        0.0
    } else {
        value as f64 / max_value as f64 * 100.0
    };
    format!("{:.0}%", width.clamp(4.0, 100.0))
}

pub(crate) fn percent_width(value: f64) -> String {
    format!("{:.0}%", (value * 100.0).clamp(4.0, 100.0))
}

pub(crate) fn ratio(value: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64
    }
}

pub(crate) fn status_tone(status: &str) -> &'static str {
    let normalized = status.to_ascii_lowercase();
    if normalized.contains("fail")
        || normalized.contains("error")
        || normalized.contains("breach")
        || normalized.contains("blocked")
        || normalized.contains("high")
    {
        "danger"
    } else if normalized.contains("warn")
        || normalized.contains("pending")
        || normalized.contains("review")
        || normalized.contains("medium")
    {
        "warning"
    } else if normalized.contains("ready")
        || normalized.contains("active")
        || normalized.contains("ok")
        || normalized.contains("pass")
        || normalized.contains("good")
    {
        "success"
    } else {
        "neutral"
    }
}

pub(crate) fn count_by<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0_u32) += 1;
    }
    map_counts_label(&counts)
}

pub(crate) fn text_input(label: &'static str, state: &UseStateHandle<String>) -> Html {
    html! {
        <label>
            {label}
            <input
                value={(**state).clone()}
                oninput={{
                    let state = state.clone();
                    Callback::from(move |event: InputEvent| {
                        state.set(event.target_unchecked_into::<HtmlInputElement>().value());
                    })
                }}
            />
        </label>
    }
}

pub(crate) fn approval_summary(approvals: &[AgentApprovalView]) -> String {
    if approvals.is_empty() {
        return "none".into();
    }
    approvals
        .iter()
        .map(|approval| {
            format!(
                "{} {}:{} by {} at {} evidence={} reason={}",
                approval.approval_id,
                approval.proposed_action,
                approval.decision,
                approval.approver,
                approval.created_at.as_deref().unwrap_or("unknown"),
                approval.evidence_refs.len(),
                approval.reason
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn approval_count_label(approvals: &[AgentApprovalView]) -> String {
    if approvals.is_empty() {
        "none".into()
    } else {
        format!("{} approval records", approvals.len())
    }
}
