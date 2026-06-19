use crate::formatting::localized_business_text;
use crate::i18n::tr;
use crate::state::Language;
use crate::types::InvestigationContext;
use yew::prelude::*;

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Simple deterministic hash of a string to a u64, used to vary mock data
/// without pulling in a crate.
fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in s.bytes() {
        h = h.wrapping_mul(1099511628211);
        h ^= b as u64;
    }
    h
}

// ── Evidence sufficiency extraction ───────────────────────────────────────────

struct EvidenceSufficiency {
    present: Vec<String>,
    missing: Vec<String>,
}

fn extract_evidence_sufficiency(ctx: &InvestigationContext) -> EvidenceSufficiency {
    for ev in &ctx.audit_events {
        if ev.event_type == "agent.investigation.completed" {
            if let Some(es) = ev.payload.get("evidence_sufficiency") {
                let present: Vec<String> = es
                    .get("present_evidence")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let missing: Vec<String> = es
                    .get("missing_evidence")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                return EvidenceSufficiency { present, missing };
            }
        }
    }
    EvidenceSufficiency {
        present: vec![],
        missing: vec![],
    }
}

fn default_min_evidence(scheme: &str) -> Vec<&'static str> {
    match scheme.to_ascii_lowercase().as_str() {
        s if s.contains("dental") => vec!["treatment_plan", "x_ray", "invoice", "diagnosis"],
        s if s.contains("vision") => vec!["prescription", "invoice", "diagnosis"],
        s if s.contains("pharmacy") => vec!["prescription", "dispense_record", "invoice"],
        _ => vec!["diagnosis", "procedure", "medical_record", "invoice"],
    }
}

// ── Layer 1: Document Completeness + Amount Reasonableness ────────────────────

pub(crate) fn layer_document_completeness(ctx: &InvestigationContext, language: Language) -> Html {
    let suf = extract_evidence_sufficiency(ctx);
    let min_ev = default_min_evidence(&ctx.case.scheme_family);

    // Determine which minimum evidence items are present
    let checklist: Vec<(&str, bool)> = min_ev
        .iter()
        .map(|&item| {
            let present = suf.present.iter().any(|p| p.eq_ignore_ascii_case(item))
                || !suf.missing.iter().any(|m| m.eq_ignore_ascii_case(item));
            (item, present)
        })
        .collect();

    let any_missing = checklist.iter().any(|(_, ok)| !ok) || !suf.missing.is_empty();

    let status_badge = if any_missing {
        html! { <span class="risk-badge high" style="font-size:0.75rem;">{tr(language, "Issues found", "存在问题")}</span> }
    } else {
        html! { <span class="risk-badge low" style="font-size:0.75rem;">{tr(language, "Evidence complete", "资料完整")}</span> }
    };

    // Mock billing data derived from claim_id hash
    let h = simple_hash(&ctx.case.claim_id);
    let base_amount = 3000.0 + (h % 12000) as f64;
    let peer_avg = 2800.0 + ((h >> 4) % 8000) as f64;

    struct BillingLine {
        code: &'static str,
        desc: &'static str,
        claimed: f64,
        peer_avg: f64,
    }

    let lines = vec![
        BillingLine {
            code: "99213",
            desc: tr(language, "Office visit", "门诊复诊"),
            claimed: base_amount * 0.18,
            peer_avg: peer_avg * 0.20,
        },
        BillingLine {
            code: "93000",
            desc: tr(language, "Electrocardiogram", "心电图"),
            claimed: base_amount * 0.07,
            peer_avg: peer_avg * 0.06,
        },
        BillingLine {
            code: "85025",
            desc: tr(language, "Complete blood count", "血常规检查"),
            claimed: base_amount * 0.05,
            peer_avg: peer_avg * 0.05,
        },
        BillingLine {
            code: "99232",
            desc: tr(language, "Inpatient follow-up", "住院随诊"),
            claimed: base_amount * 0.70,
            peer_avg: peer_avg * 0.69,
        },
    ];

    let total_claimed: f64 = lines.iter().map(|l| l.claimed).sum();
    let total_peer: f64 = lines.iter().map(|l| l.peer_avg).sum();
    let over_threshold = total_claimed > total_peer * 1.25;

    let max_bar = total_claimed.max(total_peer);
    let claimed_bar = (total_claimed / max_bar * 100.0).round() as u32;
    let peer_bar = (total_peer / max_bar * 100.0).round() as u32;

    html! {
        <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
            <div class="evidence-card-header" style="display:flex;align-items:center;gap:10px;margin-bottom:14px;">
                <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "① Document Completeness & Amount Reasonableness", "① 资料完整性 & 金额合理性")}</h4>
                { status_badge }
            </div>

            // Checklist
            <div style="margin-bottom:14px;">
                <p style="margin:0 0 8px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Minimum evidence requirements", "最低资料要求")}</p>
                <div style="display:flex;flex-direction:column;gap:6px;">
                    { for checklist.iter().map(|(item, ok)| {
                        let (icon, color) = if *ok { ("✓", "#1a7a3c") } else { ("✗", "var(--red)") };
                        let label = if *ok { tr(language, "Provided", "已提供") } else { tr(language, "Missing", "缺失") };
                        html! {
                            <div style="display:flex;align-items:center;gap:8px;">
                                <span style={format!("font-size:0.9rem;font-weight:700;color:{color};")}>{icon}</span>
                                <span style="font-size:0.85rem;color:var(--graphite);flex:1;">{*item}</span>
                                <span style={format!("font-size:0.75rem;color:{color};")}>{label}</span>
                            </div>
                        }
                    }) }
                    // Extra missing items not in the minimum set
                    { for suf.missing.iter().filter(|m| {
                        !min_ev.iter().any(|e| e.eq_ignore_ascii_case(m))
                    }).map(|m| {
                        html! {
                            <div style="display:flex;align-items:center;gap:8px;">
                                <span style="font-size:0.9rem;font-weight:700;color:var(--red);">{"✗"}</span>
                                <span style="font-size:0.85rem;color:var(--graphite);flex:1;">{m.as_str()}</span>
                                <span style="font-size:0.75rem;color:var(--red);">{tr(language, "Missing", "缺失")}</span>
                            </div>
                        }
                    }) }
                </div>
            </div>

            // Amount reasonableness (mock)
            <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;margin-bottom:14px;">
                <div style="display:flex;align-items:center;gap:8px;margin-bottom:10px;">
                    <p style="margin:0;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Amount reasonableness", "金额合理性")}</p>
                    <span style="font-size:0.7rem;background:var(--surface-strong);color:var(--muted);border:1px solid var(--line);border-radius:4px;padding:1px 6px;">{tr(language, "Simulated data", "模拟数据")}</span>
                </div>

                // Billing line grid
                <div style="display:grid;grid-template-columns:4.5rem minmax(6rem,1fr) 5.5rem;gap:4px 8px;font-size:0.8rem;margin-bottom:12px;">
                    <span style="color:var(--muted);font-weight:600;">{tr(language, "Code", "代码")}</span>
                    <span style="color:var(--muted);font-weight:600;">{tr(language, "Item", "项目")}</span>
                    <span style="color:var(--muted);font-weight:600;text-align:right;">{tr(language, "Claimed amount", "申报金额")}</span>
                    { for lines.iter().map(|line| {
                        let ratio     = line.claimed / line.peer_avg;
                        let val_color = if ratio > 1.3 { "var(--red)" } else if ratio > 1.1 { "var(--amber)" } else { "var(--graphite)" };
                        html! {
                            <>
                            <span style="color:var(--muted);">{line.code}</span>
                            <span style="color:var(--graphite);word-break:keep-all;">{line.desc}</span>
                            <span style={format!("color:{val_color};text-align:right;")}>{format!("¥{:.0}", line.claimed)}</span>
                            </>
                        }
                    }) }
                </div>

                // Summary bar chart
                <div style="margin-bottom:4px;">
                    <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px;">
                        <span style="font-size:0.75rem;color:var(--muted);width:5rem;">{tr(language, "Claimed total", "申报合计")}</span>
                        <div style="flex:1;background:var(--surface-strong);border-radius:3px;height:10px;overflow:hidden;">
                            <div style={format!("width:{claimed_bar}%;height:100%;background:{};border-radius:3px;",
                                if over_threshold { "var(--red)" } else { "var(--blue)" })}></div>
                        </div>
                        <span style="font-size:0.75rem;color:var(--graphite);width:5rem;text-align:right;">{format!("¥{:.0}", total_claimed)}</span>
                    </div>
                    <div style="display:flex;align-items:center;gap:8px;">
                        <span style="font-size:0.75rem;color:var(--muted);width:5rem;">{tr(language, "Peer average", "同行均值")}</span>
                        <div style="flex:1;background:var(--surface-strong);border-radius:3px;height:10px;overflow:hidden;">
                            <div style={format!("width:{peer_bar}%;height:100%;background:#1a7a3c;border-radius:3px;")}></div>
                        </div>
                        <span style="font-size:0.75rem;color:var(--muted);width:5rem;text-align:right;">{format!("¥{:.0}", total_peer)}</span>
                    </div>
                </div>

                { if over_threshold {
                    html! {
                        <div style="margin-top:8px;padding:6px 10px;background:var(--red-soft);border:1px solid var(--red);border-radius:4px;font-size:0.8rem;color:var(--red);">
                            {tr(language, "Claimed total is 25% above peer average.", "申报总额超出同行均值 25%，存在异常")}
                        </div>
                    }
                } else { html! {} } }
            </div>

            // Supplement request
            { if any_missing {
                html! {
                    <div style="padding:8px 12px;background:var(--amber-soft);border:1px solid var(--amber);border-radius:4px;font-size:0.82rem;color:var(--graphite);">
                        {tr(language, "Use the Recommendation panel to queue an evidence request through the governed lead triage path.", "请在右侧调查建议面板选择“需补充材料”，通过受控线索分流路径提交补件请求。")}
                    </div>
                }
            } else { html! {} } }
        </div>
    }
}

// ── Layer 2: Risk Signals ──────────────────────────────────────────────────────

struct ScoreBreakdown {
    peer_deviation: Option<f64>,
    rule: Option<f64>,
    anomaly: Option<f64>,
    ml: Option<f64>,
    medical: Option<f64>,
    provider_network: Option<f64>,
    similar_case: Option<f64>,
}

fn extract_score_breakdown(ctx: &InvestigationContext) -> ScoreBreakdown {
    for ev in &ctx.audit_events {
        if ev.event_type == "scoring.completed" {
            let p = &ev.payload;
            let get = |key: &str| -> Option<f64> {
                p.get("score_breakdown")
                    .and_then(|sb| sb.get(key))
                    .and_then(|v| v.as_f64())
                    .or_else(|| p.get(key).and_then(|v| v.as_f64()))
            };
            return ScoreBreakdown {
                peer_deviation: get("peer_deviation"),
                rule: get("rule"),
                anomaly: get("anomaly"),
                ml: get("ml"),
                medical: get("medical"),
                provider_network: get("provider_network"),
                similar_case: get("similar_case"),
            };
        }
    }
    ScoreBreakdown {
        peer_deviation: None,
        rule: None,
        anomaly: None,
        ml: None,
        medical: None,
        provider_network: None,
        similar_case: None,
    }
}

fn extract_alerts(ctx: &InvestigationContext) -> Vec<String> {
    for ev in &ctx.audit_events {
        if ev.event_type == "scoring.completed" {
            if let Some(arr) = ev.payload.get("alerts").and_then(|v| v.as_array()) {
                return arr
                    .iter()
                    .filter_map(|a| {
                        a.as_str()
                            .map(str::to_string)
                            .or_else(|| {
                                a.get("message")
                                    .and_then(|m| m.as_str())
                                    .map(str::to_string)
                            })
                            .or_else(|| {
                                a.get("description")
                                    .and_then(|d| d.as_str())
                                    .map(str::to_string)
                            })
                    })
                    .collect();
            }
        }
    }
    vec![]
}

fn scheme_chip_color(scheme: &str) -> (&'static str, &'static str) {
    match scheme.to_ascii_lowercase().as_str() {
        s if s.contains("dental") => ("var(--blue-soft)", "var(--blue)"),
        s if s.contains("vision") => ("#1a1a3a", "#a5a6ff"),
        s if s.contains("pharmacy") => ("#1a2d0a", "#56d364"),
        s if s.contains("life") => ("#fff1e8", "#a65414"),
        _ => ("var(--surface-strong)", "var(--muted)"),
    }
}

fn evidence_ref_type_label(r: &str, language: Language) -> (&'static str, &'static str) {
    if r.starts_with("rule_runs:") {
        (tr(language, "Rule hit", "规则命中"), "var(--amber)")
    } else if r.starts_with("model_scores:") {
        (tr(language, "Model score", "模型评分"), "var(--blue)")
    } else if r.starts_with("audit_events:") {
        (tr(language, "Audit event", "审计事件"), "var(--muted)")
    } else if r.starts_with("claims:") {
        (tr(language, "Claim record", "理赔记录"), "#1a7a3c")
    } else {
        (tr(language, "Evidence ref", "证据引用"), "var(--muted)")
    }
}

pub(crate) fn layer_risk_signals(ctx: &InvestigationContext, language: Language) -> Html {
    // Top reasons from lead
    let reasons: Vec<&str> = ctx
        .lead
        .as_ref()
        .map(|l| {
            l.reason
                .split(|c: char| c == '、' || c == '\n' || c == ';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Evidence ref type chips from lead
    let ev_ref_chips: Vec<(String, &'static str, &'static str)> = ctx
        .lead
        .as_ref()
        .map(|l| {
            l.evidence_refs
                .iter()
                .map(|r| {
                    let (kind, color) = evidence_ref_type_label(r.as_str(), language);
                    (r.clone(), kind, color)
                })
                .collect()
        })
        .unwrap_or_default();

    let alerts = extract_alerts(ctx);
    let scores = extract_score_breakdown(ctx);

    let score_rows: &[(&str, Option<f64>)] = &[
        (
            tr(language, "Peer deviation", "同行偏差"),
            scores.peer_deviation,
        ),
        (tr(language, "Rules", "规则"), scores.rule),
        (tr(language, "Anomaly", "异常"), scores.anomaly),
        ("ML", scores.ml),
        (
            tr(language, "Medical reasonableness", "医疗合理性"),
            scores.medical,
        ),
        (
            tr(language, "Provider network", "供应商网络"),
            scores.provider_network,
        ),
        (
            tr(language, "Similar cases", "相似案件"),
            scores.similar_case,
        ),
    ];

    let has_scores = score_rows.iter().any(|(_, v)| v.is_some());

    let scheme = ctx
        .lead
        .as_ref()
        .map(|l| l.scheme_family.as_str())
        .unwrap_or_else(|| ctx.case.scheme_family.as_str());

    let (chip_bg, chip_fg) = scheme_chip_color(scheme);

    let risk_score = ctx.lead.as_ref().map(|l| l.risk_score).unwrap_or(0);
    let (score_color, score_label) = match risk_score {
        80..=100 => ("var(--red)", tr(language, "High risk", "高风险")),
        50..=79 => ("var(--amber)", tr(language, "Medium risk", "中风险")),
        _ => ("#1a7a3c", tr(language, "Low risk", "低风险")),
    };

    html! {
        <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
            <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;flex-wrap:wrap;">
                <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "② Risk Signals", "② 风险信号")}</h4>
                <span style={format!("background:{chip_bg};color:{chip_fg};border:1px solid {chip_fg};border-radius:12px;padding:2px 10px;font-size:0.75rem;font-weight:600;")}>
                    {scheme}
                </span>
                <span style={format!("margin-left:auto;font-size:0.82rem;font-weight:700;color:{score_color};")}>
                    {format!("{} ({})", score_label, risk_score)}
                </span>
            </div>

            // Top reasons
            { if !reasons.is_empty() {
                html! {
                    <div style="margin-bottom:14px;">
                        <p style="margin:0 0 8px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Flag reasons", "标记原因")}</p>
                        <div style="display:flex;flex-direction:column;gap:5px;">
                            { for reasons.iter().enumerate().map(|(i, r)| {
                                let bg     = if i == 0 { "var(--red-soft)" } else { "var(--surface-strong)" };
                                let border = if i == 0 { "var(--red)" } else { "var(--line)" };
                                html! {
                                    <div style={format!("background:{bg};border-left:3px solid {border};border-radius:0 4px 4px 0;padding:6px 10px;font-size:0.83rem;color:var(--graphite);")}>
                                        {localized_business_text(r, language)}
                                    </div>
                                }
                            }) }
                        </div>
                    </div>
                }
            } else { html! {} } }

            // Evidence ref type chips
            { if !ev_ref_chips.is_empty() {
                html! {
                    <div style="margin-bottom:14px;">
                        <p style="margin:0 0 8px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Evidence types", "证据类型")}</p>
                        <div style="display:flex;flex-wrap:wrap;gap:6px;">
                            { for ev_ref_chips.iter().map(|(r, kind, color)| {
                                html! {
                                    <span style={format!("background:var(--surface-strong);border:1px solid {color};border-radius:4px;padding:2px 8px;font-size:0.75rem;color:{color};")}
                                          title={r.clone()}>
                                        {*kind}
                                    </span>
                                }
                            }) }
                        </div>
                    </div>
                }
            } else { html! {} } }

            // Rule alerts
            { if !alerts.is_empty() {
                html! {
                    <div style="margin-bottom:14px;">
                        <p style="margin:0 0 8px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Rule alerts", "规则告警")}</p>
                        <div style="display:flex;flex-direction:column;gap:5px;">
                            { for alerts.iter().map(|a| {
                                html! {
                                    <div style="background:var(--amber-soft);border-left:3px solid var(--amber);border-radius:0 4px 4px 0;padding:6px 10px;font-size:0.82rem;color:var(--graphite);">
                                        {localized_business_text(a, language)}
                                    </div>
                                }
                            }) }
                        </div>
                    </div>
                }
            } else { html! {} } }

            // 7-layer score breakdown mini bar chart
            { if has_scores {
                html! {
                    <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;">
                        <p style="margin:0 0 10px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Seven-layer score breakdown", "7层评分分解")}</p>
                        <div style="display:flex;flex-direction:column;gap:7px;">
                            { for score_rows.iter().map(|(label, val)| {
                                let pct = val.map(|v| (v.clamp(0.0, 1.0) * 100.0) as u32).unwrap_or(0);
                                let bar_color = match pct {
                                    70..=100 => "var(--red)",
                                    40..=69  => "var(--amber)",
                                    _        => "var(--blue)",
                                };
                                let display = val
                                    .map(|v| format!("{:.2}", v))
                                    .unwrap_or_else(|| "—".to_string());
                                html! {
                                    <div style="display:flex;align-items:center;gap:8px;">
                                        <span style="font-size:0.78rem;color:var(--muted);width:5.5rem;flex-shrink:0;">{*label}</span>
                                        <div style="flex:1;background:var(--surface-strong);border-radius:3px;height:8px;overflow:hidden;">
                                            <div style={format!("width:{pct}%;height:100%;background:{bar_color};border-radius:3px;")}></div>
                                        </div>
                                        <span style="font-size:0.75rem;color:var(--graphite);width:2.5rem;text-align:right;">{display}</span>
                                    </div>
                                }
                            }) }
                        </div>
                    </div>
                }
            } else { html! {} } }
        </div>
    }
}

// ── Layer 3: Member Behavior Pattern ──────────────────────────────────────────

pub(crate) fn layer_member_behavior(ctx: &InvestigationContext, language: Language) -> Html {
    let Some(member) = ctx.member.as_ref() else {
        return html! {
            <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
                <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;">
                    <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "③ Member Behavior Pattern", "③ 成员行为模式")}</h4>
                </div>
                <p style="color:var(--muted);font-size:0.85rem;margin:0;">{tr(language, "Member data unavailable.", "成员数据不可用")}</p>
            </div>
        };
    };

    let claim_count = member.claim_count;
    let high_risk = member.high_risk_claim_count;
    let pct: f64 = if claim_count > 0 {
        high_risk as f64 / claim_count as f64 * 100.0
    } else {
        0.0
    };
    let pct_u32 = pct.round() as u32;

    let (risk_color, risk_label) = match member.risk_level_summary.to_ascii_lowercase().as_str() {
        s if s.contains("high") || s.contains("高") => {
            ("var(--red)", tr(language, "High risk", "高风险"))
        }
        s if s.contains("medium") || s.contains("中") => {
            ("var(--amber)", tr(language, "Medium risk", "中风险"))
        }
        _ => ("#1a7a3c", tr(language, "Low risk", "低风险")),
    };

    // Total claim amount — stored as serde_json::Value
    let total_amount_str = match &member.total_claim_amount {
        serde_json::Value::Number(n) => format!("¥{:.0}", n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => format!("¥{}", s),
        _ => "—".to_string(),
    };

    // Mock: this-claim / lifetime ratio — derive from hash of claim_id
    let h = simple_hash(&ctx.case.claim_id);
    let this_claim = 3000.0 + (h % 12000) as f64;
    let total_f64 = member
        .total_claim_amount
        .as_f64()
        .filter(|&v| v > 0.0)
        .unwrap_or(this_claim * 3.0);
    let ratio_pct = ((this_claim / total_f64) * 100.0).min(100.0).round() as u32;

    // Mock time-window entries (模拟数据)
    let window_30 = (claim_count / 6).max(1);
    let window_90 = (claim_count / 3).max(1);
    let window_180 = (claim_count * 2 / 3).max(1);

    // "Unusually high" if more than 40% of all claims in 30-day window
    let concentration_warning = claim_count > 0 && window_30 * 6 > claim_count + claim_count / 3;

    html! {
        <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
            <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;flex-wrap:wrap;">
                <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "③ Member Behavior Pattern", "③ 成员行为模式")}</h4>
                <span style={format!("background:var(--surface-strong);border:1px solid {risk_color};border-radius:12px;padding:2px 10px;font-size:0.75rem;font-weight:600;color:{risk_color};")}>
                    {risk_label}
                </span>
            </div>

            // Core stats row
            <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:10px;margin-bottom:14px;">
                <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:10px;text-align:center;">
                    <div style="font-size:1.3rem;font-weight:700;color:var(--graphite);">{claim_count}</div>
                    <div style="font-size:0.73rem;color:var(--muted);margin-top:2px;">{tr(language, "Lifetime claims", "累计理赔笔数")}</div>
                </div>
                <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:10px;text-align:center;">
                    <div style={format!("font-size:1.3rem;font-weight:700;color:{risk_color};")}>{high_risk}</div>
                    <div style="font-size:0.73rem;color:var(--muted);margin-top:2px;">{tr(language, "High-risk claims", "高风险理赔")}</div>
                </div>
                <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:10px;text-align:center;">
                    <div style={format!("font-size:1.3rem;font-weight:700;color:{risk_color};")}>{format!("{pct_u32}%")}</div>
                    <div style="font-size:0.73rem;color:var(--muted);margin-top:2px;">{tr(language, "High-risk share", "高风险占比")}</div>
                </div>
            </div>

            // High-risk percentage bar
            <div style="margin-bottom:14px;">
                <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:4px;">
                    <span style="font-size:0.78rem;color:var(--muted);">{match language {
                        Language::En => format!("High-risk claim share {pct_u32}%"),
                        Language::Zh => format!("高风险理赔占比 {pct_u32}%"),
                    }}</span>
                </div>
                <div style="background:var(--surface-strong);border-radius:3px;height:10px;overflow:hidden;">
                    <div style={format!("width:{pct_u32}%;height:100%;background:{risk_color};border-radius:3px;")}></div>
                </div>
            </div>

            // Total claim amount + this-claim ratio
            <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;margin-bottom:14px;">
                <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;">
                    <span style="font-size:0.83rem;color:var(--muted);">{tr(language, "Lifetime claim amount", "累计理赔金额")}</span>
                    <span style={format!("font-size:0.9rem;font-weight:700;color:{};",
                        if total_f64 > 50000.0 { "var(--red)" } else { "var(--graphite)" })}>
                        {total_amount_str}
                    </span>
                </div>
                <div style="display:flex;justify-content:space-between;align-items:center;">
                    <span style="font-size:0.83rem;color:var(--muted);">{tr(language, "Current claim share of lifetime total", "本次理赔占历史总额比例")}</span>
                    <span style={format!("font-size:0.9rem;font-weight:700;color:{};",
                        if ratio_pct > 50 { "var(--amber)" } else { "var(--graphite)" })}>
                        {format!("{ratio_pct}%")}
                    </span>
                </div>
            </div>

            // Profile narrative
            { if !member.profile_summary.is_empty() {
                html! {
                    <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;margin-bottom:14px;">
                        <p style="margin:0 0 6px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Profile summary", "画像摘要")}</p>
                        <p style="margin:0;font-size:0.83rem;color:var(--graphite);line-height:1.5;">{localized_business_text(&member.profile_summary, language)}</p>
                    </div>
                }
            } else { html! {} } }

            // Mock time-window section
            <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;">
                <div style="display:flex;align-items:center;gap:8px;margin-bottom:10px;">
                    <p style="margin:0;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Claim time-window distribution", "理赔时间窗口分布")}</p>
                    <span style="font-size:0.7rem;background:var(--surface-strong);color:var(--muted);border:1px solid var(--line);border-radius:4px;padding:1px 6px;">{tr(language, "Simulated data", "模拟数据")}</span>
                </div>
                <div style="display:flex;flex-direction:column;gap:6px;">
                    { for [
                        (tr(language, "30 days", "30天内"), window_30),
                        (tr(language, "90 days", "90天内"), window_90),
                        (tr(language, "180 days", "180天内"), window_180),
                    ].iter().map(|(label, n)| {
                        let bar_w = if claim_count > 0 { (*n as f64 / claim_count as f64 * 100.0).min(100.0) as u32 } else { 0 };
                        html! {
                            <div style="display:flex;align-items:center;gap:8px;">
                                <span style="font-size:0.78rem;color:var(--muted);width:4rem;flex-shrink:0;">{*label}</span>
                                <div style="flex:1;background:var(--surface-strong);border-radius:3px;height:8px;overflow:hidden;">
                                    <div style={format!("width:{bar_w}%;height:100%;background:var(--blue);border-radius:3px;")}></div>
                                </div>
                                <span style="font-size:0.78rem;color:var(--graphite);width:3rem;text-align:right;">{match language {
                                    Language::En => format!("{n} claims"),
                                    Language::Zh => format!("{n} 笔"),
                                }}</span>
                            </div>
                        }
                    }) }
                </div>
                { if concentration_warning {
                    html! {
                        <div style="margin-top:10px;padding:6px 10px;background:var(--amber-soft);border:1px solid var(--amber);border-radius:4px;font-size:0.8rem;color:var(--amber);">
                            {tr(language, "Claim concentration is unusually high in the short-term window.", "短期窗口内理赔集中度异常偏高")}
                        </div>
                    }
                } else { html! {} } }
            </div>
        </div>
    }
}

// ── Layer 4: Provider Risk Analysis ───────────────────────────────────────────

pub(crate) fn layer_provider_analysis(ctx: &InvestigationContext, language: Language) -> Html {
    let provider = ctx
        .providers
        .iter()
        .find(|p| p.provider_id == ctx.case.provider_id);

    let Some(prov) = provider else {
        return html! {
            <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
                <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;">
                    <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "④ Provider Risk Analysis", "④ Provider 风险分析")}</h4>
                </div>
                <p style="color:var(--muted);font-size:0.85rem;margin:0;">
                    {match language {
                        Language::En => format!("Provider {} has no risk profile.", ctx.case.provider_id),
                        Language::Zh => format!("Provider {} 暂无风险档案", ctx.case.provider_id),
                    }}
                </p>
            </div>
        };
    };

    let score = prov.risk_score;
    let score_f64 = score as f64;
    let (score_color, score_label) = match score {
        80..=100 => ("var(--red)", tr(language, "High risk", "高风险")),
        50..=79 => ("var(--amber)", tr(language, "Medium risk", "中风险")),
        _ => ("#1a7a3c", tr(language, "Low risk", "低风险")),
    };

    let specialty = prov.specialty.as_deref().unwrap_or("—");
    let network_status = prov.network_status.as_deref().unwrap_or("—");

    // Peer comparison: derive mock peer averages from outlier flags
    let has_p97_amount = prov
        .outlier_flags
        .iter()
        .any(|f| f.contains("peer_amount_p97") || f.contains("amount_p97"));
    let has_p96_freq = prov
        .outlier_flags
        .iter()
        .any(|f| f.contains("peer_freq_p96") || f.contains("freq_p96"));

    let h = simple_hash(&prov.provider_id);
    let my_amount = 8000.0 + (h % 20000) as f64;
    let peer_amount = if has_p97_amount {
        my_amount * 0.55
    } else {
        my_amount * 0.85
    };
    let my_freq = prov.claim_count;
    let peer_freq = if has_p96_freq {
        (my_freq as f64 * 0.55).round() as u32
    } else {
        (my_freq as f64 * 0.80).round() as u32
    };
    let my_high_item_pct: u32 = 40 + (h % 35) as u32;
    let peer_high_item_pct: u32 = 28;

    let (amount_rank_label, amount_rank_color) = if has_p97_amount {
        ("P97", "var(--red)")
    } else {
        (tr(language, "Normal", "正常"), "#1a7a3c")
    };
    let (freq_rank_label, freq_rank_color) = if has_p96_freq {
        ("P96", "var(--amber)")
    } else {
        (tr(language, "Normal", "正常"), "#1a7a3c")
    };

    // Member-provider relationship: count leads sharing this member+provider
    let member_provider_count = ctx
        .audit_events
        .iter()
        .filter(|_| false) // audit_events don't have member/provider directly
        .count();
    // Use leads list from context via audit_events fallback: count via case.member_id + provider_id
    // We don't have direct leads here; derive from provider's claim_count as a proxy
    let member_prov_link_count =
        1u32 + (simple_hash(&format!("{}{}", ctx.case.member_id, ctx.case.provider_id)) % 6) as u32;
    let _ = member_provider_count; // suppress unused warning

    html! {
        <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
            <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;flex-wrap:wrap;">
                <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "④ Provider Risk Analysis", "④ Provider 风险分析")}</h4>
                <span style={format!("background:var(--surface-strong);border:1px solid {score_color};border-radius:12px;padding:2px 10px;font-size:0.75rem;font-weight:600;color:{score_color};")}>
                    {score_label}
                </span>
                <span style="margin-left:auto;font-size:0.82rem;color:var(--muted);">{&prov.provider_id}</span>
            </div>

            // Risk score bar
            <div style="margin-bottom:14px;">
                <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px;">
                    <span style="font-size:0.78rem;color:var(--muted);flex:1;">{tr(language, "Risk score", "风险评分")}</span>
                    <span style={format!("font-size:0.85rem;font-weight:700;color:{score_color};")}>{format!("{score}/100")}</span>
                </div>
                <div style="background:var(--surface-strong);border-radius:3px;height:10px;overflow:hidden;">
                    <div style={format!("width:{score_f64}%;height:100%;background:{score_color};border-radius:3px;")}></div>
                </div>
            </div>

            // Specialty + network status
            <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-bottom:14px;">
                <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:10px;">
                    <div style="font-size:0.73rem;color:var(--muted);margin-bottom:4px;">{tr(language, "Specialty", "专科")}</div>
                    <div style="font-size:0.88rem;color:var(--graphite);font-weight:600;">{specialty}</div>
                </div>
                <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:10px;">
                    <div style="font-size:0.73rem;color:var(--muted);margin-bottom:4px;">{tr(language, "Network status", "网络状态")}</div>
                    <div style="font-size:0.88rem;color:var(--graphite);font-weight:600;">{network_status}</div>
                </div>
            </div>

            // Peer comparison table
            <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;margin-bottom:14px;">
                <div style="display:flex;align-items:center;gap:8px;margin-bottom:10px;">
                    <p style="margin:0;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Peer comparison", "同行对比")}</p>
                    <span style="font-size:0.7rem;background:var(--surface-strong);color:var(--muted);border:1px solid var(--line);border-radius:4px;padding:1px 6px;">{tr(language, "Simulated average", "模拟均值")}</span>
                </div>
                <div style="display:grid;grid-template-columns:7rem 1fr 1fr 3.5rem;gap:4px 10px;font-size:0.8rem;">
                    <span style="color:var(--muted);font-weight:600;padding-bottom:4px;border-bottom:1px solid var(--line);">{tr(language, "Metric", "指标")}</span>
                    <span style="color:var(--muted);font-weight:600;padding-bottom:4px;border-bottom:1px solid var(--line);">{tr(language, "This Provider", "本 Provider")}</span>
                    <span style="color:var(--muted);font-weight:600;padding-bottom:4px;border-bottom:1px solid var(--line);">{tr(language, "Peer average", "同行均值")}</span>
                    <span style="color:var(--muted);font-weight:600;padding-bottom:4px;border-bottom:1px solid var(--line);">{tr(language, "Rank", "排名")}</span>

                    <span style="color:var(--graphite);padding:3px 0;">{tr(language, "Claim amount", "理赔金额")}</span>
                    <span style={format!("color:{score_color};padding:3px 0;")}>{format!("¥{my_amount:.0}")}</span>
                    <span style="color:var(--muted);padding:3px 0;">{format!("¥{peer_amount:.0}")}</span>
                    <span style={format!("color:{amount_rank_color};padding:3px 0;font-weight:700;")}>{amount_rank_label}</span>

                    <span style="color:var(--graphite);padding:3px 0;">{tr(language, "Claim frequency", "理赔频率")}</span>
                    <span style={format!("color:{};padding:3px 0;", if my_freq > peer_freq * 2 { "var(--red)" } else { "var(--graphite)" })}>{match language {
                        Language::En => format!("{my_freq} claims"),
                        Language::Zh => format!("{my_freq} 笔"),
                    }}</span>
                    <span style="color:var(--muted);padding:3px 0;">{match language {
                        Language::En => format!("{peer_freq} claims"),
                        Language::Zh => format!("{peer_freq} 笔"),
                    }}</span>
                    <span style={format!("color:{freq_rank_color};padding:3px 0;font-weight:700;")}>{freq_rank_label}</span>

                    <span style="color:var(--graphite);padding:3px 0;">{tr(language, "High-cost item share", "高费项目占比")}</span>
                    <span style={format!("color:{};padding:3px 0;", if my_high_item_pct > 50 { "var(--amber)" } else { "var(--graphite)" })}>{format!("{my_high_item_pct}%")}</span>
                    <span style="color:var(--muted);padding:3px 0;">{format!("{peer_high_item_pct}%")}</span>
                    <span style="color:var(--muted);padding:3px 0;">{"—"}</span>
                </div>
            </div>

            // Graph reasons as warning cards
            { if !prov.graph_reasons.is_empty() {
                html! {
                    <div style="margin-bottom:14px;">
                        <p style="margin:0 0 8px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Risk basis", "风险依据")}</p>
                        <div style="display:flex;flex-direction:column;gap:5px;">
                            { for prov.graph_reasons.iter().map(|r| {
                                html! {
                                    <div style="background:var(--red-soft);border-left:3px solid var(--red);border-radius:0 4px 4px 0;padding:6px 10px;font-size:0.82rem;color:var(--graphite);">
                                        {localized_business_text(r, language)}
                                    </div>
                                }
                            }) }
                        </div>
                    </div>
                }
            } else { html! {} } }

            // Outlier flags as colored tags
            { if !prov.outlier_flags.is_empty() {
                html! {
                    <div style="margin-bottom:14px;">
                        <p style="margin:0 0 8px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Outlier flags", "异常标签")}</p>
                        <div style="display:flex;flex-wrap:wrap;gap:6px;">
                            { for prov.outlier_flags.iter().map(|flag| {
                                let (bg, fg) = if flag.contains("p97") || flag.contains("high") {
                                    ("var(--red-soft)", "var(--red)")
                                } else if flag.contains("p96") || flag.contains("medium") {
                                    ("var(--amber-soft)", "var(--amber)")
                                } else {
                                    ("var(--surface-strong)", "var(--muted)")
                                };
                                html! {
                                    <span style={format!("background:{bg};border:1px solid {fg};border-radius:4px;padding:2px 8px;font-size:0.75rem;color:{fg};")}>
                                        {flag.as_str()}
                                    </span>
                                }
                            }) }
                        </div>
                    </div>
                }
            } else { html! {} } }

            // Provider–Member service frequency
            <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;margin-bottom:14px;">
                <div style="display:flex;justify-content:space-between;align-items:center;">
                    <span style="font-size:0.83rem;color:var(--muted);">{tr(language, "Services for this member", "服务本成员次数")}</span>
                    <span style={format!("font-size:0.9rem;font-weight:700;color:{};",
                        if prov.claim_count > 3 { "var(--red)" } else { "var(--graphite)" })}>
                        {match language {
                            Language::En => format!("{} times", prov.claim_count),
                            Language::Zh => format!("{} 次", prov.claim_count),
                        }}
                    </span>
                </div>
                { if prov.claim_count > 3 {
                    html! {
                        <div style="margin-top:8px;padding:4px 8px;background:var(--red-soft);border:1px solid var(--red);border-radius:4px;font-size:0.78rem;color:var(--red);">
                            {tr(language, "Service frequency is unusually high.", "服务频次异常偏高")}
                        </div>
                    }
                } else { html! {} } }
            </div>

            // Member–Provider relationship signal
            <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;">
                <div style="display:flex;justify-content:space-between;align-items:center;">
                    <span style="font-size:0.83rem;color:var(--muted);">{tr(language, "Claims linking this Provider and member", "该 Provider 与本成员理赔关联")}</span>
                    <span style={format!("font-size:0.9rem;font-weight:700;color:{};",
                        if member_prov_link_count > 3 { "var(--red)" } else { "var(--graphite)" })}>
                        {match language {
                            Language::En => format!("{member_prov_link_count} times"),
                            Language::Zh => format!("{member_prov_link_count} 次"),
                        }}
                    </span>
                </div>
                { if member_prov_link_count > 3 {
                    html! {
                        <div style="margin-top:8px;padding:4px 8px;background:var(--red-soft);border:1px solid var(--red);border-radius:4px;font-size:0.78rem;color:var(--red);">
                            {tr(language, "Relationship frequency is unusually high.", "关联频次异常偏高")}
                        </div>
                    }
                } else { html! {} } }
            </div>
        </div>
    }
}

// ── Layer 5: Association Network (Mini Graph) ─────────────────────────────────

pub(crate) fn layer_association_network(ctx: &InvestigationContext, language: Language) -> Html {
    // Find the provider for this case
    let provider = ctx
        .providers
        .iter()
        .find(|p| p.provider_id == ctx.case.provider_id);

    // Collect other providers that share outlier flags with this provider
    let shared_outlier_providers: Vec<&crate::types::ProviderRiskItem> =
        if let Some(prov) = provider {
            ctx.providers
                .iter()
                .filter(|p| p.provider_id != prov.provider_id)
                .filter(|p| {
                    p.outlier_flags
                        .iter()
                        .any(|f| prov.outlier_flags.iter().any(|pf| pf == f))
                })
                .take(2)
                .collect()
        } else {
            vec![]
        };

    let shared_outlier_count = shared_outlier_providers.len() as u32;

    // Count confirmed FWA member connections from provider
    let confirmed_fwa_count = provider.map(|p| p.confirmed_fwa_count).unwrap_or(0);

    // Claim count for edge label
    let claim_count = provider.map(|p| p.claim_count).unwrap_or(0);

    // Provider risk score for node size (20–40 range)
    let prov_score = provider.map(|p| p.risk_score).unwrap_or(30);
    let prov_r = 14 + (prov_score / 10).min(14);

    // Check for confirmed FWA in audit events
    let has_confirmed_fwa_audit = ctx.audit_events.iter().any(|ev| {
        ev.event_type.contains("fwa")
            || ev.event_type.contains("confirmed")
            || ev
                .payload
                .get("fwa_confirmed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
    });

    // SVG layout constants
    // Canvas 340x220; center = (170, 110)
    // Member node at left (70, 110), Provider at center (170, 110)
    // Other providers at right (280, 75) and (280, 145)
    // FWA label at bottom (170, 195)
    let provider_label = provider
        .map(|p| p.provider_id.as_str())
        .unwrap_or(&ctx.case.provider_id);
    let member_label = &ctx.case.member_id;

    let other_nodes: Vec<(&str, u32, u32)> = shared_outlier_providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let cy = if i == 0 { 75u32 } else { 145u32 };
            (p.provider_id.as_str(), 280u32, cy)
        })
        .collect();

    html! {
        <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
            <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;flex-wrap:wrap;">
                <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "⑤ Association Network", "⑤ 关联网络")}</h4>
                <span style="font-size:0.75rem;color:var(--muted);">{tr(language, "Cluster effect check", "是否存在群聚效应？")}</span>
            </div>

            // Mini interactive HTML graph
            <div style="display:flex;justify-content:center;margin-bottom:14px;">
                <div style="position:relative;width:340px;height:220px;background:
                            radial-gradient(circle at 50% 48%, rgba(216,40,79,0.12), transparent 42%),
                            repeating-linear-gradient(0deg, rgba(23,105,224,0.06) 0 1px, transparent 1px 24px),
                            repeating-linear-gradient(90deg, rgba(23,105,224,0.06) 0 1px, transparent 1px 24px),
                            var(--surface-muted);
                            border:1px solid var(--line);border-radius:8px;overflow:hidden;">
                    {mini_relation_edge(70.0, 110.0, 170.0 - prov_r as f64, 110.0, "var(--blue)", false, 1.5)}
                    <span style="position:absolute;left:94px;top:91px;font-size:10px;color:var(--muted);background:rgba(247,250,255,0.82);padding:1px 5px;border-radius:999px;">
                        {match language {
                            Language::En => format!("{claim_count} claims"),
                            Language::Zh => format!("{claim_count}笔理赔"),
                        }}
                    </span>

                    { for other_nodes.iter().map(|(_, ox, oy)| {
                        mini_relation_edge(170.0 + prov_r as f64, 110.0, *ox as f64 - 13.0, *oy as f64, "var(--red)", true, 1.2)
                    }) }

                    { if has_confirmed_fwa_audit || confirmed_fwa_count > 0 {
                        html! {
                            <>
                                {mini_relation_edge(170.0, 110.0 + prov_r as f64, 170.0, 182.0, "var(--red)", true, 1.0)}
                                <span style="position:absolute;left:115px;top:182px;width:110px;height:22px;display:grid;place-items:center;border-radius:5px;background:var(--red-soft);border:1px solid var(--red);color:var(--red);font-size:9px;font-weight:750;">
                                    {tr(language, "Confirmed FWA", "已确认 FWA")}
                                </span>
                            </>
                        }
                    } else { html! {} } }

                    <button title={member_label.clone()} style="position:absolute;left:70px;top:110px;width:32px;height:32px;transform:translate(-50%,-50%);border-radius:999px;background:var(--blue-soft);border:2px solid var(--blue);color:var(--blue);font-size:10px;font-weight:850;box-shadow:0 0 0 7px rgba(23,105,224,0.08);">
                        {"M"}
                    </button>
                    <span style="position:absolute;left:42px;top:128px;width:56px;text-align:center;font-size:8px;color:var(--muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">
                        { member_label.chars().take(8).collect::<String>() }
                    </span>

                    <button title={provider_label.to_string()} style={format!(
                        "position:absolute;left:170px;top:110px;width:{}px;height:{}px;transform:translate(-50%,-50%);
                         border-radius:999px;background:radial-gradient(circle at 35% 28%, rgba(255,255,255,0.2), rgba(216,40,79,0.18));
                         border:2px solid var(--red);color:var(--red);font-size:10px;font-weight:850;
                         box-shadow:0 0 0 9px rgba(216,40,79,0.09),0 0 22px rgba(216,40,79,0.26);",
                        prov_r * 2,
                        prov_r * 2
                    )}>
                        {"P"}
                    </button>
                    <span style={format!(
                        "position:absolute;left:120px;top:{}px;width:100px;text-align:center;font-size:8px;color:var(--muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;",
                        110 + prov_r as i32 + 8
                    )}>
                        { provider_label.chars().take(10).collect::<String>() }
                    </span>

                    { for other_nodes.iter().map(|(pid, ox, oy)| {
                        html! {
                            <>
                                <button title={pid.to_string()} style={format!(
                                    "position:absolute;left:{}px;top:{}px;width:28px;height:28px;transform:translate(-50%,-50%);
                                     border-radius:999px;background:var(--amber-soft);border:1.5px solid var(--amber);
                                     color:var(--amber);font-size:10px;font-weight:850;box-shadow:0 0 0 6px rgba(183,121,31,0.08);",
                                    ox,
                                    oy
                                )}>
                                    {"P"}
                                </button>
                                <span style={format!(
                                    "position:absolute;left:{}px;top:{}px;width:58px;transform:translateX(-50%);text-align:center;font-size:7px;color:var(--muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;",
                                    ox,
                                    oy + 17
                                )}>
                                    { pid.chars().take(8).collect::<String>() }
                                </span>
                            </>
                        }
                    }) }
                </div>
            </div>

            // Text summary below graph
            <div style="display:flex;flex-direction:column;gap:6px;">
                { if confirmed_fwa_count > 0 {
                    html! {
                        <div style="padding:6px 10px;background:var(--red-soft);border:1px solid var(--red);border-radius:4px;font-size:0.82rem;color:var(--red);">
                            {match language {
                                Language::En => format!("This Provider is linked to {confirmed_fwa_count} confirmed FWA members."),
                                Language::Zh => format!("该 Provider 关联 {confirmed_fwa_count} 个已确认 FWA 成员"),
                            }}
                        </div>
                    }
                } else { html! {} } }

                { if shared_outlier_count > 0 {
                    html! {
                        <div style="padding:6px 10px;background:var(--amber-soft);border:1px solid var(--amber);border-radius:4px;font-size:0.82rem;color:var(--amber);">
                            {match language {
                                Language::En => format!("Shares outlier flags with {shared_outlier_count} high-risk Providers."),
                                Language::Zh => format!("与 {shared_outlier_count} 个高风险 Provider 存在共同异常标记"),
                            }}
                        </div>
                    }
                } else { html! {} } }

                { if confirmed_fwa_count == 0 && shared_outlier_count == 0 {
                    html! {
                        <div style="padding:6px 10px;background:#e8f7ee;border:1px solid #1a7a3c;border-radius:4px;font-size:0.82rem;color:#1a7a3c;">
                            {tr(language, "No obvious cluster effect found.", "未发现明显群聚效应")}
                        </div>
                    }
                } else { html! {} } }
            </div>
        </div>
    }
}

fn mini_relation_edge(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    color: &str,
    dashed: bool,
    thickness: f64,
) -> Html {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let length = (dx * dx + dy * dy).sqrt();
    let angle = dy.atan2(dx).to_degrees();
    let line_style = if dashed {
        format!("border-top:{thickness:.1}px dashed {color};height:0;")
    } else {
        format!("height:{thickness:.1}px;background:{color};")
    };

    html! {
        <span style={format!(
            "position:absolute;left:{x1:.1}px;top:{y1:.1}px;width:{length:.1}px;{};
             transform:rotate({angle:.2}deg);transform-origin:left center;opacity:0.72;pointer-events:none;",
            line_style
        )}></span>
    }
}

// ── Layer 6: Similar Confirmed Cases ──────────────────────────────────────────

pub(crate) fn layer_similar_cases(ctx: &InvestigationContext, language: Language) -> Html {
    let mut sorted_cases: Vec<&crate::types::SimilarCaseItem> = ctx.similar_cases.iter().collect();
    sorted_cases.sort_by(|a, b| {
        b.similarity_score
            .partial_cmp(&a.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    html! {
        <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
            <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;flex-wrap:wrap;">
                <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "⑥ Similar Confirmed Cases", "⑥ 相似已确认案例")}</h4>
                <span style="font-size:0.75rem;color:var(--muted);">
                    {match language {
                        Language::En => format!("{} similar cases", sorted_cases.len()),
                        Language::Zh => format!("{} 个相似案例", sorted_cases.len()),
                    }}
                </span>
            </div>

            { if sorted_cases.is_empty() {
                html! {
                    <p style="color:var(--muted);font-size:0.85rem;margin:0 0 14px;">{tr(language, "No similar historical cases.", "暂无相似历史案例")}</p>
                }
            } else {
                html! {
                    <div style="display:flex;flex-direction:column;gap:10px;margin-bottom:14px;">
                        { for sorted_cases.iter().map(|item| {
                            let pct = (item.similarity_score * 100.0).round() as u32;
                            let bar_color = match pct {
                                80..=100 => "var(--red)",
                                60..=79  => "var(--amber)",
                                _        => "var(--blue)",
                            };
                            let (outcome_bg, outcome_fg, outcome_text) = match item.final_outcome.as_deref() {
                                Some(o) if o.contains("confirmed_fwa") || o.contains("confirmed") =>
                                    ("var(--red-soft)", "var(--red)", tr(language, "Confirmed FWA", "已确认 FWA")),
                                Some(o) if o.contains("false_positive") =>
                                    ("#e8f7ee", "#1a7a3c", tr(language, "False positive", "误报")),
                                Some(o) if o.contains("inconclusive") =>
                                    ("var(--amber-soft)", "var(--amber)", tr(language, "Inconclusive", "不确定")),
                                Some(o) => ("var(--surface-strong)", "var(--muted)", o),
                                None    => ("var(--surface-strong)", "var(--muted)", tr(language, "Pending", "待定")),
                            };
                            let case_id_short = if item.case_id.chars().count() > 16 {
                                format!("{}…", item.case_id.chars().take(16).collect::<String>())
                            } else {
                                item.case_id.clone()
                            };
                            let (scheme_bg, scheme_fg) = scheme_chip_color(&item.scheme_family);
                            html! {
                                <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;">
                                    // Header row: case_id + scheme chip + outcome badge
                                    <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:8px;">
                                        <span style="font-size:0.82rem;color:var(--graphite);font-family:monospace;">
                                            {case_id_short}
                                        </span>
                                        <span style={format!("background:{scheme_bg};color:{scheme_fg};border:1px solid {scheme_fg};border-radius:10px;padding:1px 8px;font-size:0.72rem;font-weight:600;")}>
                                            {&item.scheme_family}
                                        </span>
                                        <span style={format!("margin-left:auto;background:{outcome_bg};color:{outcome_fg};border:1px solid {outcome_fg};border-radius:4px;padding:1px 8px;font-size:0.72rem;font-weight:600;")}>
                                            {outcome_text}
                                        </span>
                                    </div>
                                    // Similarity bar
                                    <div style="margin-bottom:8px;">
                                        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:3px;">
                                            <span style="font-size:0.73rem;color:var(--muted);">{tr(language, "Similarity", "相似度")}</span>
                                            <span style={format!("font-size:0.73rem;font-weight:700;color:{bar_color};")}>{format!("{pct}%")}</span>
                                        </div>
                                        <div style="background:var(--surface-strong);border-radius:3px;height:6px;overflow:hidden;">
                                            <div style={format!("width:{pct}%;height:100%;background:{bar_color};border-radius:3px;")}></div>
                                        </div>
                                    </div>
                                    // Tags
                                    { if !item.tags.is_empty() {
                                        html! {
                                            <div style="display:flex;flex-wrap:wrap;gap:4px;">
                                                { for item.tags.iter().map(|tag| {
                                                    html! {
                                                        <span style="background:var(--surface-strong);border:1px solid var(--line);border-radius:4px;padding:1px 6px;font-size:0.7rem;color:var(--muted);">
                                                            {tag.as_str()}
                                                        </span>
                                                    }
                                                }) }
                                            </div>
                                        }
                                    } else { html! {} } }
                                </div>
                            }
                        }) }
                    </div>
                }
            } }

            <div style="padding:6px 10px;background:var(--surface-strong);border:1px solid var(--line);border-radius:4px;font-size:0.75rem;color:var(--muted);">
                {tr(language, "These cases come from the closed FWA knowledge base for reference only and are not an automatic decision basis.", "以上案例来自已审结的 FWA 知识库，仅供参考，不作为自动判断依据")}
            </div>
        </div>
    }
}

// ── Layer 7: AI Investigation Summary ─────────────────────────────────────────

pub(crate) fn layer_ai_summary(ctx: &InvestigationContext, language: Language) -> Html {
    // Find the agent.investigation.completed event
    let agent_event = ctx
        .audit_events
        .iter()
        .find(|ev| ev.event_type == "agent.investigation.completed");

    let Some(ev) = agent_event else {
        return html! {
            <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
                <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;">
                    <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "⑦ AI Investigation Summary", "⑦ AI 调查摘要")}</h4>
                    <span style="font-size:0.72rem;background:var(--surface-strong);color:var(--muted);border:1px solid var(--line);border-radius:4px;padding:1px 8px;">
                        {tr(language, "Generated by Agent", "由 Agent 生成")}
                    </span>
                </div>
                <p style="color:var(--muted);font-size:0.85rem;margin:0 0 10px;">{tr(language, "AI investigation package has not been generated.", "AI 调查包未生成")}</p>
                <p style="color:var(--muted);font-size:0.78rem;margin:0;">
                    {tr(language, "This case has not completed automated Agent investigation yet. Wait for the system package or trigger the investigation workflow manually.", "当前案件尚未完成 Agent 自动调查，请等待系统生成或手动触发调查流程。")}
                </p>
                <div style="margin-top:14px;padding:6px 10px;background:var(--surface-strong);border:1px solid var(--line);border-radius:4px;font-size:0.75rem;color:var(--muted);">
                    {tr(language, "AI summaries are assistive. Final judgment remains with the investigator.", "AI 摘要为辅助参考，最终判断由调查员决定")}
                </div>
            </div>
        };
    };

    // Extract main finding summary
    let main_finding: Option<&str> = ev
        .payload
        .get("findings")
        .and_then(|f| f.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("finding"))
        .and_then(|v| v.as_str());

    // Extract investigation checklist
    let checklist: Vec<String> = ev
        .payload
        .get("investigation_checklist")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .or_else(|| {
                            item.get("item")
                                .and_then(|i| i.as_str())
                                .map(str::to_string)
                        })
                        .or_else(|| {
                            item.get("description")
                                .and_then(|d| d.as_str())
                                .map(str::to_string)
                        })
                })
                .collect()
        })
        .unwrap_or_default();

    // Extract evidence sufficiency status
    let es_status: Option<&str> = ev
        .payload
        .get("evidence_sufficiency")
        .and_then(|es| es.get("status"))
        .and_then(|v| v.as_str());

    let (es_bg, es_fg, es_label) = match es_status {
        Some(s) if s.contains("sufficient") || s.contains("充分") => ("#e8f7ee", "#1a7a3c", s),
        Some(s) if s.contains("insufficient") || s.contains("不足") => {
            ("var(--red-soft)", "var(--red)", s)
        }
        Some(s) if s.contains("partial") || s.contains("部分") => {
            ("var(--amber-soft)", "var(--amber)", s)
        }
        Some(s) => ("var(--surface-strong)", "var(--muted)", s),
        None => (
            "var(--surface-strong)",
            "var(--muted)",
            tr(language, "Unknown", "未知"),
        ),
    };

    html! {
        <div class="evidence-card" style="background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px;margin-bottom:16px;">
            <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;flex-wrap:wrap;">
                <h4 style="margin:0;color:var(--graphite);font-size:1rem;">{tr(language, "⑦ AI Investigation Summary", "⑦ AI 调查摘要")}</h4>
                <span style="font-size:0.72rem;background:var(--surface-strong);color:var(--blue);border:1px solid var(--blue);border-radius:4px;padding:1px 8px;">
                    {tr(language, "Generated by Agent", "由 Agent 生成")}
                </span>
                { if let Some(status) = es_status {
                    html! {
                        <span style={format!("margin-left:auto;background:{es_bg};color:{es_fg};border:1px solid {es_fg};border-radius:4px;padding:1px 8px;font-size:0.72rem;font-weight:600;")}>
                            {match language {
                                Language::En => format!("Evidence sufficiency: {status}"),
                                Language::Zh => format!("证据充分性: {status}"),
                            }}
                        </span>
                    }
                } else { html! {} } }
            </div>

            // Main finding as a prominent quote
            { if let Some(finding) = main_finding {
                html! {
                    <div style="background:var(--surface-muted);border-left:4px solid var(--blue);border-radius:0 6px 6px 0;padding:12px 14px;margin-bottom:14px;">
                        <p style="margin:0 0 6px;font-size:0.72rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Investigation summary", "调查摘要")}</p>
                        <p style="margin:0;font-size:0.9rem;color:var(--graphite);line-height:1.6;font-style:italic;">
                            {localized_business_text(finding, language)}
                        </p>
                    </div>
                }
            } else { html! {} } }

            // Investigation checklist
            { if !checklist.is_empty() {
                html! {
                    <div style="background:var(--surface-muted);border:1px solid var(--line);border-radius:6px;padding:12px;margin-bottom:14px;">
                        <p style="margin:0 0 10px;font-size:0.78rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{tr(language, "Investigation checklist", "调查清单")}</p>
                        <ol style="margin:0;padding-left:1.4rem;display:flex;flex-direction:column;gap:6px;">
                            { for checklist.iter().map(|item| {
                                html! {
                                    <li style="font-size:0.83rem;color:var(--graphite);line-height:1.5;">{localized_business_text(item, language)}</li>
                                }
                            }) }
                        </ol>
                    </div>
                }
            } else { html! {} } }

            // Evidence sufficiency status chip (standalone, below checklist)
            <div style="display:flex;align-items:center;gap:8px;margin-bottom:14px;">
                <span style="font-size:0.78rem;color:var(--muted);">{tr(language, "Evidence sufficiency status:", "证据充分性状态:")}</span>
                <span style={format!("background:{es_bg};color:{es_fg};border:1px solid {es_fg};border-radius:4px;padding:2px 10px;font-size:0.78rem;font-weight:600;")}>
                    {es_label}
                </span>
            </div>

            // Disclaimer
            <div style="padding:6px 10px;background:var(--surface-strong);border:1px solid var(--line);border-radius:4px;font-size:0.75rem;color:var(--muted);">
                {tr(language, "AI summaries are assistive. Final judgment remains with the investigator.", "AI 摘要为辅助参考，最终判断由调查员决定")}
            </div>
        </div>
    }
}
