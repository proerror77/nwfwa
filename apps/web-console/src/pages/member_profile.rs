use crate::api::*;
use crate::data_helpers::*;
use crate::formatting::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use crate::ui_helpers::text_input;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[function_component(MemberProfilePage)]
pub fn member_profile_page() -> Html {
    let api_key = use_api_key();
    let member_id = use_state(|| "MBR-0287".to_string());
    let profile_state = use_state(|| ApiState::<MemberProfileSummary>::Idle);

    let load_profile = {
        let api_key = api_key.clone();
        let member_id = member_id.clone();
        let profile_state = profile_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let member_id = (*member_id).clone();
            let profile_state = profile_state.clone();
            profile_state.set(ApiState::Loading);
            spawn_local(async move {
                profile_state.set(match get_member_profile_summary(api_key, member_id).await {
                    Ok(p) => ApiState::Ready(p),
                    Err(e) => ApiState::Failed(e),
                });
            });
        })
    };

    let refresh = {
        let load_profile = load_profile.clone();
        Callback::from(move |_| load_profile.emit(()))
    };

    {
        let load_profile = load_profile.clone();
        use_effect_with((), move |_| {
            load_profile.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"成员画像"}</h2>
                    <p>{"查询投保人的历史理赔、风险评级与证据链，辅助调查员做出审核决策。"}</p>
                </div>
                <span class="status-pill">{"成员风险档案"}</span>
            </div>

            <section class="panel">
                <div class="form-grid" style="display:grid;grid-template-columns:1fr auto;gap:10px;align-items:end;">
                    {text_input("成员 ID", &member_id)}
                    <button
                        onclick={refresh}
                        disabled={matches!(&*profile_state, ApiState::Loading)}
                        style="height:42px;padding:0 20px;"
                    >
                        {if matches!(&*profile_state, ApiState::Loading) { "查询中..." } else { "查询" }}
                    </button>
                </div>
            </section>

            <MemberProfileView state={(*profile_state).clone()} />
        </section>
    }
}

// ── Risk level helpers ────────────────────────────────────────────────────────

fn risk_tone(summary: &str) -> &'static str {
    let s = summary.to_ascii_lowercase();
    if s.contains("high") || s.contains("critical") || s.contains("高") {
        "danger"
    } else if s.contains("medium") || s.contains("moderate") || s.contains("中") {
        "warning"
    } else {
        "success"
    }
}

fn risk_label(summary: &str) -> &'static str {
    let s = summary.to_ascii_lowercase();
    if s.contains("high") || s.contains("critical") || s.contains("高") {
        "高风险历史"
    } else if s.contains("medium") || s.contains("moderate") || s.contains("中") {
        "中等风险"
    } else {
        "低风险"
    }
}

fn risk_icon(summary: &str) -> &'static str {
    let s = summary.to_ascii_lowercase();
    if s.contains("high") || s.contains("critical") || s.contains("高") {
        "⚠️"
    } else if s.contains("medium") || s.contains("moderate") || s.contains("中") {
        "🟡"
    } else {
        "✓"
    }
}

// ── Main view ─────────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct MemberProfileProps {
    state: ApiState<MemberProfileSummary>,
}

#[function_component(MemberProfileView)]
fn member_profile_view(props: &MemberProfileProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {
            <section class="panel">
                <p class="empty">{"输入成员 ID 后点击查询，查看该投保人的风险档案。"}</p>
            </section>
        },
        ApiState::Loading => html! {
            <section class="panel">
                <p class="empty">{"正在加载成员档案..."}</p>
            </section>
        },
        ApiState::Failed(e) => html! {
            <section class="panel"><p class="error">{e}</p></section>
        },
        ApiState::Ready(p) => html! { <ProfileDetail profile={p.clone()} /> },
    }
}

// ── Profile detail ────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct ProfileDetailProps {
    profile: MemberProfileSummary,
}

#[function_component(ProfileDetail)]
fn profile_detail(props: &ProfileDetailProps) -> Html {
    let p = &props.profile;
    let tone = risk_tone(&p.risk_level_summary);
    let label = risk_label(&p.risk_level_summary);
    let icon = risk_icon(&p.risk_level_summary);
    let amount = format!("{} {}", display_value(&p.total_claim_amount), p.currency);
    let high_risk_pct = if p.claim_count > 0 {
        (p.high_risk_claim_count * 100) / p.claim_count
    } else {
        0
    };

    html! {
        <>
        // ── Identity + risk verdict ────────────────────────────────────────
        <section class="panel" style="margin-bottom:12px;">
            <div style="display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:12px;">
                <div style="display:flex;align-items:center;gap:16px;">
                    // Avatar circle
                    <div style="
                        width:52px;height:52px;border-radius:50%;
                        background:linear-gradient(135deg,var(--graphite-2),var(--teal));
                        display:flex;align-items:center;justify-content:center;
                        color:#fff;font-size:18px;font-weight:800;flex-shrink:0;
                    ">
                        {p.member_id.chars().next().unwrap_or('?').to_string()}
                    </div>
                    <div>
                        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.06em;color:var(--muted);margin-bottom:3px;">
                            {"成员 ID"}
                        </div>
                        <div style="font-size:22px;font-weight:800;color:var(--graphite);">
                            {&p.member_id}
                        </div>
                    </div>
                </div>
                // Risk verdict badge
                <div style={format!(
                    "display:flex;align-items:center;gap:8px;padding:10px 18px;border-radius:10px;{}",
                    match tone {
                        "danger"  => "background:var(--red-soft);border:1.5px solid #ffc2d0;",
                        "warning" => "background:var(--amber-soft);border:1.5px solid #f7d87a;",
                        _         => "background:#e8f7ee;border:1.5px solid #b7e4c4;",
                    }
                )}>
                    <span style="font-size:20px;">{icon}</span>
                    <div>
                        <div style={format!(
                            "font-size:13px;font-weight:700;{}",
                            match tone {
                                "danger"  => "color:var(--red);",
                                "warning" => "color:var(--amber);",
                                _         => "color:#1a7a3c;",
                            }
                        )}>
                            {label}
                        </div>
                        <div style="font-size:11px;color:var(--muted);">
                            {format!("{} / {} 笔理赔为高风险", p.high_risk_claim_count, p.claim_count)}
                        </div>
                    </div>
                </div>
            </div>
        </section>

        // ── KPI cards ──────────────────────────────────────────────────────
        <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:12px;margin-bottom:12px;">
            {kpi_card("历史理赔", &p.claim_count.to_string(), "笔", "#1769e0")}
            {kpi_card("总理赔金额", &amount, "", "#0f7b8c")}
            {kpi_card("保单数量", &p.policy_count.to_string(), "张", "#5f6f85")}
            {kpi_card_pct("高风险占比", high_risk_pct, tone)}
        </div>

        // ── Profile narrative + signal cards ──────────────────────────────
        <div style="display:grid;grid-template-columns:1fr 340px;gap:12px;margin-bottom:12px;">
            // Narrative
            <section class="panel">
                <h3 style="margin:0 0 12px;font-size:13px;text-transform:uppercase;letter-spacing:.05em;color:var(--muted);">
                    {"调查员摘要"}
                </h3>
                <p style="font-size:14px;line-height:1.65;color:var(--graphite);margin:0;">
                    {&p.profile_summary}
                </p>
                if let Some(ref cid) = p.latest_claim_id {
                    <div style="margin-top:14px;padding:10px 14px;background:var(--surface-muted);border-radius:7px;border-left:3px solid var(--blue);">
                        <span style="font-size:11px;color:var(--muted);">{"最近理赔单"}</span>
                        <div style="font-size:14px;font-weight:700;color:var(--graphite);">{cid}</div>
                    </div>
                }
            </section>

            // Risk signals
            <section class="panel" style="display:flex;flex-direction:column;gap:8px;">
                <h3 style="margin:0 0 4px;font-size:13px;text-transform:uppercase;letter-spacing:.05em;color:var(--muted);">
                    {"风险信号"}
                </h3>
                {signal_row("理赔笔数", &p.claim_count.to_string(), "neutral")}
                {signal_row("高风险理赔", &p.high_risk_claim_count.to_string(),
                    if p.high_risk_claim_count > 0 { "danger" } else { "success" })}
                {signal_row("累计理赔金额", &amount,
                    if high_risk_pct > 50 { "danger" } else if high_risk_pct > 20 { "warning" } else { "neutral" })}
                {signal_row("保单数", &p.policy_count.to_string(), "neutral")}
            </section>
        </div>

        // ── Evidence refs ──────────────────────────────────────────────────
        if !p.evidence_refs.is_empty() {
            <section class="panel">
                <h3 style="margin:0 0 10px;font-size:13px;text-transform:uppercase;letter-spacing:.05em;color:var(--muted);">
                    {format!("证据链（{} 条）", p.evidence_refs.len())}
                </h3>
                <div style="display:flex;flex-wrap:wrap;gap:6px;">
                    {for p.evidence_refs.iter().map(|r| html! {
                        <span style="
                            padding:3px 10px;border-radius:20px;
                            font-size:11px;font-family:monospace;
                            background:var(--surface-strong);
                            border:1px solid var(--line);
                            color:var(--muted);
                        ">{r}</span>
                    })}
                </div>
            </section>
        }
        </>
    }
}

// ── Helper components ─────────────────────────────────────────────────────────

fn kpi_card(label: &str, value: &str, unit: &str, color: &str) -> Html {
    html! {
        <section class="panel" style="text-align:center;padding:16px 12px;">
            <div style={format!("font-size:28px;font-weight:800;color:{color};line-height:1;margin-bottom:4px;")}>
                {value}
                if !unit.is_empty() {
                    <span style="font-size:13px;font-weight:500;margin-left:3px;color:var(--muted);">{unit}</span>
                }
            </div>
            <div style="font-size:11px;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">
                {label}
            </div>
        </section>
    }
}

fn kpi_card_pct(label: &str, pct: u32, tone: &str) -> Html {
    let color = match tone {
        "danger" => "#d8284f",
        "warning" => "#b7791f",
        _ => "#1a7a3c",
    };
    html! {
        <section class="panel" style="text-align:center;padding:16px 12px;">
            <div style={format!("font-size:28px;font-weight:800;color:{color};line-height:1;margin-bottom:4px;")}>
                {format!("{pct}%")}
            </div>
            <div style="font-size:11px;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">
                {label}
            </div>
            // Mini progress bar
            <div style="margin-top:8px;height:4px;background:var(--line);border-radius:2px;overflow:hidden;">
                <div style={format!(
                    "height:100%;border-radius:2px;background:{color};width:{}%;transition:width .4s;",
                    pct.min(100)
                )}></div>
            </div>
        </section>
    }
}

fn signal_row(label: &str, value: &str, tone: &str) -> Html {
    let (bg, border, text_color) = match tone {
        "danger" => ("var(--red-soft)", "var(--red)", "var(--red)"),
        "warning" => ("var(--amber-soft)", "var(--amber)", "var(--amber)"),
        "success" => ("#e8f7ee", "#1a7a3c", "#1a7a3c"),
        _ => (
            "var(--surface-muted)",
            "var(--line-strong)",
            "var(--graphite)",
        ),
    };
    html! {
        <div style={format!(
            "display:flex;justify-content:space-between;align-items:center;
             padding:9px 12px;border-radius:7px;border-left:3px solid {border};background:{bg};"
        )}>
            <span style="font-size:12px;color:var(--muted);">{label}</span>
            <strong style={format!("font-size:13px;font-weight:700;color:{text_color};")}>{value}</strong>
        </div>
    }
}
