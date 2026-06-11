use crate::api::*;
use crate::formatting::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use std::f64::consts::PI;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;

// ── Page ─────────────────────────────────────────────────────────────────────

#[function_component(ProviderRiskPage)]
pub fn provider_risk_page() -> Html {
    let api_key = use_api_key();
    let summary_state = use_state(|| ApiState::<ProviderRiskSummary>::Idle);
    let selected_id = use_state(|| Option::<String>::None);

    let load = {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        let selected_id = selected_id.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            let selected_id = selected_id.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                let result = get_provider_risk_summary(api_key).await;
                if let Ok(ref s) = result {
                    if let Some(top) = s.providers.iter().max_by_key(|p| p.risk_score) {
                        selected_id.set(Some(top.provider_id.clone()));
                    }
                }
                summary_state.set(match result {
                    Ok(s) => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
        })
    };

    let refresh = {
        let load = load.clone();
        Callback::from(move |_| load.emit(()))
    };

    {
        let load = load.clone();
        use_effect_with((), move |_| {
            load.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Provider 风险图谱"}</h2>
                    <p>{"可视化 Provider 网络关系、同行偏离信号与图谱风险原因。点击节点查看详细分析。"}</p>
                </div>
                <span class="status-pill">{"Provider 网络分析"}</span>
            </div>

            <section class="panel" style="padding:10px 16px;">
                <div style="display:flex;align-items:center;justify-content:space-between;gap:12px;">
                    <p class="empty" style="margin:0;font-size:13px;">
                        {"使用已配置的 Provider 风险数据，展示网络图谱与异常信号。"}
                    </p>
                    <button
                        onclick={refresh}
                        disabled={matches!(&*summary_state, ApiState::Loading)}
                        style="white-space:nowrap;flex-shrink:0;"
                    >
                        {if matches!(&*summary_state, ApiState::Loading) { "刷新中..." } else { "刷新图谱" }}
                    </button>
                </div>
            </section>

            {match &*summary_state {
                ApiState::Idle => html! {
                    <section class="panel">
                        <p class="empty">{"点击刷新加载 Provider 风险图谱。"}</p>
                    </section>
                },
                ApiState::Loading => html! {
                    <section class="panel">
                        <p class="empty">{"正在加载 Provider 风险数据..."}</p>
                    </section>
                },
                ApiState::Failed(e) => html! {
                    <section class="panel"><p class="error">{e}</p></section>
                },
                ApiState::Ready(summary) => html! {
                    <ProviderRiskView
                        summary={summary.clone()}
                        selected_id={(*selected_id).clone()}
                        on_select={{
                            let selected_id = selected_id.clone();
                            Callback::from(move |id: String| selected_id.set(Some(id)))
                        }}
                    />
                },
            }}
        </section>
    }
}

// ── Main view ─────────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct ProviderRiskViewProps {
    summary: ProviderRiskSummary,
    selected_id: Option<String>,
    on_select: Callback<String>,
}

#[function_component(ProviderRiskView)]
fn provider_risk_view(props: &ProviderRiskViewProps) -> Html {
    let s = &props.summary;
    let selected = props.selected_id.as_ref()
        .and_then(|id| s.providers.iter().find(|p| p.provider_id == *id));

    html! {
        <div style="display:grid;grid-template-columns:1fr 380px;gap:16px;align-items:start;">
            <div style="display:flex;flex-direction:column;gap:12px;">
                // KPI strip
                <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:10px;">
                    {kpi_mini("Provider 总数", &s.provider_count.to_string(), "#5f6f85")}
                    {kpi_mini("高风险", &s.high_risk_count.to_string(), "#d8284f")}
                    {kpi_mini("需审核", &s.review_required_count.to_string(), "#b7791f")}
                </div>
                // Network graph
                <section class="panel" style="padding:0;overflow:hidden;">
                    <div style="padding:11px 16px 9px;border-bottom:1px solid var(--line);
                                display:flex;align-items:center;gap:10px;">
                        <span style="font-size:12px;font-weight:700;text-transform:uppercase;
                                     letter-spacing:.05em;color:var(--muted);">
                            {"Provider 风险网络图"}
                        </span>
                        <span style="font-size:11px;color:var(--faint);">
                            {"节点大小 = 理赔量　颜色 = 风险等级　点击节点查看原因"}
                        </span>
                    </div>
                    <ProviderNetworkGraph
                        providers={s.providers.clone()}
                        selected_id={props.selected_id.clone()}
                        on_select={props.on_select.clone()}
                    />
                </section>
                // Provider list
                <section class="panel" style="padding:0;overflow:hidden;">
                    <div style="padding:9px 16px;border-bottom:1px solid var(--line);
                                font-size:11px;font-weight:700;text-transform:uppercase;
                                letter-spacing:.05em;color:var(--muted);">
                        {"全部 Provider"}
                    </div>
                    {for s.providers.iter().map(|p| {
                        let on_select = props.on_select.clone();
                        let pid = p.provider_id.clone();
                        let is_active = props.selected_id.as_deref() == Some(&p.provider_id);
                        provider_list_row(p, is_active,
                            Callback::from(move |_| on_select.emit(pid.clone())))
                    })}
                </section>
            </div>

            // Detail panel
            if let Some(p) = selected {
                <ProviderDetailPanel provider={p.clone()} />
            } else {
                <section class="panel" style="position:sticky;top:16px;padding:48px 24px;text-align:center;">
                    <p class="empty">{"点击图谱中的节点查看 Provider 详情与风险原因"}</p>
                </section>
            }
        </div>
    }
}

// ── SVG network graph ─────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct NetworkGraphProps {
    providers: Vec<ProviderRiskItem>,
    selected_id: Option<String>,
    on_select: Callback<String>,
}

#[function_component(ProviderNetworkGraph)]
fn provider_network_graph(props: &NetworkGraphProps) -> Html {
    let providers = &props.providers;
    if providers.is_empty() {
        return html! { <p class="empty" style="padding:48px 24px;text-align:center;">{"无 Provider 数据"}</p> };
    }

    let w = 580.0_f64;
    let h = 400.0_f64;
    let cx = w / 2.0;
    let cy = h / 2.0;

    // Sort by risk: highest goes to center
    let mut sorted = providers.to_vec();
    sorted.sort_by(|a, b| b.risk_score.cmp(&a.risk_score));

    let center = &sorted[0];
    let orbit: Vec<&ProviderRiskItem> = sorted[1..].iter().collect();
    let n = orbit.len();

    let max_claims = providers.iter().map(|p| p.claim_count).max().unwrap_or(1).max(1);

    let node_r = |count: u32| -> f64 {
        18.0 + 18.0 * (count as f64 / max_claims as f64).sqrt()
    };

    // Orbit positions — stagger radius by risk to show proximity
    let orbit_positions: Vec<(f64, f64)> = orbit.iter().enumerate().map(|(i, p)| {
        let angle = 2.0 * PI * i as f64 / n.max(1) as f64 - PI / 2.0;
        // Higher risk = closer to center
        let r = 130.0 + (100 - p.risk_score.min(100)) as f64 * 0.5;
        (cx + r * angle.cos(), cy + r * angle.sin())
    }).collect();

    html! {
        <svg viewBox={format!("0 0 {w} {h}")}
             style="width:100%;height:400px;display:block;background:linear-gradient(180deg,#f8faff,#f3f7fd);"
             xmlns="http://www.w3.org/2000/svg">
            <defs>
                <filter id="node-glow">
                    <feGaussianBlur stdDeviation="3" result="blur"/>
                    <feMerge>
                        <feMergeNode in="blur"/>
                        <feMergeNode in="SourceGraphic"/>
                    </feMerge>
                </filter>
                <pattern id="g" width="30" height="30" patternUnits="userSpaceOnUse">
                    <path d="M30 0L0 0 0 30" fill="none" stroke="rgba(23,105,224,0.05)" stroke-width="0.5"/>
                </pattern>
            </defs>
            <rect width={w.to_string()} height={h.to_string()} fill="url(#g)"/>

            // ── Edges ─────────────────────────────────────────────────────────
            {for orbit.iter().zip(orbit_positions.iter()).map(|(p, (ox, oy))| {
                let opacity = if p.risk_score >= 70 { 0.5 } else if p.risk_score >= 40 { 0.25 } else { 0.12 };
                let stroke = node_color(&p.risk_tier, p.risk_score);
                let sw = 1.0 + (p.claim_count as f64 / max_claims as f64) * 3.5;
                let dash = if p.risk_score < 40 { "5,5" } else { "" };
                html! {
                    <line x1={cx.to_string()} y1={cy.to_string()}
                          x2={ox.to_string()} y2={oy.to_string()}
                          stroke={stroke}
                          stroke-width={format!("{sw:.1}")}
                          stroke-opacity={format!("{opacity:.2}")}
                          stroke-dasharray={dash}/>
                }
            })}

            // ── Orbit nodes ───────────────────────────────────────────────────
            {for orbit.iter().zip(orbit_positions.iter()).map(|(p, (ox, oy))| {
                let r = node_r_fn(p.claim_count, max_claims);
                let fill = node_color(&p.risk_tier, p.risk_score);
                let fill_soft = node_color_soft(&p.risk_tier, p.risk_score);
                let is_sel = props.selected_id.as_deref() == Some(&p.provider_id);
                let pid = p.provider_id.clone();
                let on_select = props.on_select.clone();

                html! {
                    <g style="cursor:pointer;"
                       onclick={Callback::from(move |_| on_select.emit(pid.clone()))}>
                        if is_sel {
                            <circle cx={ox.to_string()} cy={oy.to_string()}
                                    r={(r+8.0).to_string()}
                                    fill="none" stroke={fill}
                                    stroke-width="2" stroke-opacity="0.35"/>
                        }
                        <circle cx={ox.to_string()} cy={oy.to_string()}
                                r={r.to_string()}
                                fill={fill_soft} stroke={fill}
                                stroke-width={if is_sel { "2.5" } else { "1.5" }}
                                filter={if is_sel { "url(#node-glow)" } else { "" }}/>
                        <text x={ox.to_string()} y={oy.to_string()}
                              text-anchor="middle" dominant-baseline="central"
                              font-size="12" font-weight="700" fill={fill}>
                            {p.risk_score.to_string()}
                        </text>
                        <text x={ox.to_string()} y={(oy + r + 13.0).to_string()}
                              text-anchor="middle" font-size="10" fill="#5f6f85">
                            {trim_id(&p.provider_id)}
                        </text>
                        // Graph reason count badge
                        if !p.graph_reasons.is_empty() {
                            <circle cx={(ox + r*0.72).to_string()}
                                    cy={(oy - r*0.72).to_string()}
                                    r="9" fill={fill} stroke="white" stroke-width="1.5"/>
                            <text x={(ox + r*0.72).to_string()}
                                  y={(oy - r*0.72).to_string()}
                                  text-anchor="middle" dominant-baseline="central"
                                  font-size="8" font-weight="700" fill="white">
                                {p.graph_reasons.len().to_string()}
                            </text>
                        }
                    </g>
                }
            })}

            // ── Center node — compute outside html! then inline ────────────────
            {center_node(center, &props.selected_id, props.on_select.clone(), cx, cy, max_claims)}

            // ── Legend ────────────────────────────────────────────────────────
            <g transform="translate(10,375)">
                <circle cx="8" cy="7" r="6" fill="#d8284f"/>
                <text x="18" y="11" font-size="10" fill="#5f6f85">{"高风险"}</text>
                <circle cx="65" cy="7" r="6" fill="#b7791f"/>
                <text x="75" y="11" font-size="10" fill="#5f6f85">{"中风险"}</text>
                <circle cx="122" cy="7" r="6" fill="#1a7a3c"/>
                <text x="132" y="11" font-size="10" fill="#5f6f85">{"低风险"}</text>
                <circle cx="179" cy="7" r="7" fill="none" stroke="#888" stroke-width="1"/>
                <text x="179" y="7" text-anchor="middle" dominant-baseline="central"
                      font-size="7" fill="#888">{"N"}</text>
                <text x="190" y="11" font-size="10" fill="#5f6f85">{"图谱信号数"}</text>
                <text x="280" y="11" font-size="10" fill="#5f6f85">{"线粗 = 理赔量　近 = 高风险"}</text>
            </g>
        </svg>
    }
}

// ── Provider detail panel ─────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct DetailPanelProps {
    provider: ProviderRiskItem,
}

#[function_component(ProviderDetailPanel)]
fn provider_detail_panel(props: &DetailPanelProps) -> Html {
    let p = &props.provider;
    let fill = node_color(&p.risk_tier, p.risk_score);
    let fill_soft = node_color_soft(&p.risk_tier, p.risk_score);

    html! {
        <section class="panel" style="position:sticky;top:16px;display:flex;flex-direction:column;gap:14px;max-height:calc(100vh - 120px);overflow-y:auto;">
            // Header
            <div style={format!(
                "display:flex;align-items:center;gap:12px;padding:14px 16px;
                 margin:-16px -16px 0;background:{fill_soft};border-bottom:1px solid var(--line);"
            )}>
                <div style={format!(
                    "width:46px;height:46px;border-radius:50%;background:{fill};flex-shrink:0;
                     display:flex;align-items:center;justify-content:center;
                     color:white;font-size:16px;font-weight:800;"
                )}>
                    {p.risk_score.to_string()}
                </div>
                <div style="min-width:0;">
                    <div style="font-size:15px;font-weight:800;color:var(--graphite);overflow-wrap:anywhere;">
                        {&p.provider_id}
                    </div>
                    <div style="font-size:11px;color:var(--muted);margin-top:2px;">
                        {p.specialty.as_deref().unwrap_or("未知专科")}
                        {"  ·  "}
                        {p.network_status.as_deref().unwrap_or("—")}
                    </div>
                </div>
                if p.review_required {
                    <span style="margin-left:auto;flex-shrink:0;padding:3px 8px;
                                 background:#d8284f;color:white;border-radius:4px;
                                 font-size:11px;font-weight:700;">{"需审核"}</span>
                }
            </div>

            // Stats
            <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:8px;">
                {stat_box("理赔量", &p.claim_count.to_string(), "#1769e0")}
                {stat_box("确认 FWA", &p.confirmed_fwa_count.to_string(), "#d8284f")}
                {stat_box("误报", &p.false_positive_count.to_string(), "#5f6f85")}
            </div>

            // Risk bars
            <div style="display:flex;flex-direction:column;gap:8px;">
                {score_bar("综合风险评分", p.risk_score, fill)}
                if let Some(net) = p.network_risk_score {
                    {score_bar("图谱网络风险", net, fill)}
                }
            </div>

            // Graph reasons — the WHY
            if !p.graph_reasons.is_empty() {
                <div>
                    <div style="font-size:11px;font-weight:700;text-transform:uppercase;
                                letter-spacing:.06em;color:var(--muted);margin-bottom:8px;">
                        {format!("图谱风险原因（{} 条）", p.graph_reasons.len())}
                    </div>
                    <div style="display:flex;flex-direction:column;gap:6px;">
                        {for p.graph_reasons.iter().map(|reason| html! {
                            <div style="display:flex;gap:8px;padding:9px 11px;border-radius:7px;
                                        background:var(--red-soft);border-left:3px solid #d8284f;
                                        align-items:flex-start;">
                                <span style="color:#d8284f;font-size:14px;flex-shrink:0;line-height:1.4;">{"⚠"}</span>
                                <span style="font-size:12px;line-height:1.55;color:var(--graphite);">{reason}</span>
                            </div>
                        })}
                    </div>
                </div>
            }

            // Outlier flags
            if !p.outlier_flags.is_empty() {
                <div>
                    <div style="font-size:11px;font-weight:700;text-transform:uppercase;
                                letter-spacing:.06em;color:var(--muted);margin-bottom:8px;">
                        {"异常标记"}
                    </div>
                    <div style="display:flex;flex-wrap:wrap;gap:6px;">
                        {for p.outlier_flags.iter().map(|flag| html! {
                            <span style="padding:4px 10px;border-radius:20px;font-size:11px;
                                          font-weight:600;background:var(--amber-soft);
                                          color:var(--amber);border:1px solid #f7d87a;">
                                {fmt_flag(flag)}
                            </span>
                        })}
                    </div>
                </div>
            }

            // Review route
            if !p.review_route.is_empty() {
                <div style="padding:10px 12px;border-radius:7px;
                             background:var(--surface-strong);border:1px solid var(--line);">
                    <div style="font-size:11px;color:var(--muted);margin-bottom:3px;">{"建议处置"}</div>
                    <div style="font-size:13px;font-weight:600;color:var(--graphite);">
                        {fmt_route(&p.review_route)}
                    </div>
                </div>
            }

            // Evidence refs
            if !p.evidence_refs.is_empty() {
                <div>
                    <div style="font-size:11px;font-weight:700;text-transform:uppercase;
                                letter-spacing:.06em;color:var(--muted);margin-bottom:6px;">
                        {format!("证据链（{} 条）", p.evidence_refs.len())}
                    </div>
                    <div style="display:flex;flex-wrap:wrap;gap:5px;">
                        {for p.evidence_refs.iter().take(10).map(|r| html! {
                            <span style="padding:2px 8px;border-radius:4px;font-size:10px;
                                          font-family:monospace;background:var(--surface-muted);
                                          border:1px solid var(--line);color:var(--muted);">{r}</span>
                        })}
                    </div>
                </div>
            }
        </section>
    }
}

// ── Provider list row ─────────────────────────────────────────────────────────

fn provider_list_row(p: &ProviderRiskItem, is_active: bool, on_click: Callback<MouseEvent>) -> Html {
    let fill = node_color(&p.risk_tier, p.risk_score);
    let row_style = if is_active {
        "background:#f0f6ff;border-left:3px solid var(--blue);"
    } else {
        "border-left:3px solid transparent;"
    };
    html! {
        <div style={format!(
            "display:grid;grid-template-columns:32px 1fr auto;gap:10px;align-items:center;
             padding:10px 14px;border-bottom:1px solid var(--line);cursor:pointer;
             transition:background .12s;{row_style}"
        )} onclick={on_click}>
            <div style={format!(
                "width:30px;height:30px;border-radius:50%;background:{fill};flex-shrink:0;
                 display:flex;align-items:center;justify-content:center;
                 color:white;font-size:11px;font-weight:700;"
            )}>
                {p.risk_score.to_string()}
            </div>
            <div>
                <div style="font-size:13px;font-weight:600;color:var(--graphite);">{&p.provider_id}</div>
                <div style="font-size:11px;color:var(--muted);">
                    {p.specialty.as_deref().unwrap_or("—")}
                    {"  ·  "}
                    {format!("{} 笔理赔", p.claim_count)}
                    if !p.graph_reasons.is_empty() {
                        <span style="margin-left:6px;color:#d8284f;font-weight:600;">
                            {format!("{}个图谱信号", p.graph_reasons.len())}
                        </span>
                    }
                </div>
            </div>
            if p.review_required {
                <span style="padding:2px 7px;background:var(--red-soft);color:var(--red);
                              border-radius:4px;font-size:10px;font-weight:700;white-space:nowrap;">
                    {"需审核"}
                </span>
            }
        </div>
    }
}

// ── Small helpers ─────────────────────────────────────────────────────────────

fn kpi_mini(label: &str, value: &str, color: &str) -> Html {
    html! {
        <section class="panel" style="padding:12px 14px;text-align:center;">
            <div style={format!("font-size:22px;font-weight:800;color:{color};line-height:1;margin-bottom:3px;")}>{value}</div>
            <div style="font-size:10px;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;">{label}</div>
        </section>
    }
}

fn stat_box(label: &str, value: &str, color: &str) -> Html {
    html! {
        <div style="text-align:center;padding:8px;background:var(--surface-muted);border-radius:7px;">
            <div style={format!("font-size:18px;font-weight:800;color:{color};line-height:1;")}>{value}</div>
            <div style="font-size:10px;color:var(--muted);margin-top:2px;">{label}</div>
        </div>
    }
}

fn score_bar(label: &str, score: u8, fill: &str) -> Html {
    html! {
        <div>
            <div style="display:flex;justify-content:space-between;font-size:11px;color:var(--muted);margin-bottom:4px;">
                <span>{label}</span>
                <span style={format!("font-weight:700;color:{fill};")}>{format!("{score} / 100")}</span>
            </div>
            <div style="height:7px;background:var(--line);border-radius:4px;overflow:hidden;">
                <div style={format!("height:100%;background:{fill};border-radius:4px;width:{score}%;transition:width .5s;")}></div>
            </div>
        </div>
    }
}

fn trim_id(id: &str) -> String {
    if id.len() > 11 { format!("{}…", &id[..11]) } else { id.to_string() }
}

fn node_r_fn(count: u32, max: u32) -> f64 {
    18.0 + 18.0 * (count as f64 / max.max(1) as f64).sqrt()
}

fn center_node(
    p: &ProviderRiskItem,
    selected_id: &Option<String>,
    on_select: Callback<String>,
    cx: f64,
    cy: f64,
    max_claims: u32,
) -> Html {
    let r = node_r_fn(p.claim_count, max_claims).max(28.0);
    let fill = node_color(&p.risk_tier, p.risk_score);
    let is_sel = selected_id.as_deref() == Some(&p.provider_id);
    let pid = p.provider_id.clone();
    let score = p.risk_score.to_string();
    let label = trim_id(&p.provider_id);
    let reason_count = p.graph_reasons.len();
    let bx2 = cx + r * 0.7;
    let by2 = cy - r * 0.7;

    html! {
        <g style="cursor:pointer;"
           onclick={Callback::from(move |_| on_select.emit(pid.clone()))}>
            <circle cx={cx.to_string()} cy={cy.to_string()}
                    r={(r+14.0).to_string()}
                    fill="none" stroke={fill}
                    stroke-width="1" stroke-opacity="0.15"/>
            <circle cx={cx.to_string()} cy={cy.to_string()}
                    r={(r+7.0).to_string()}
                    fill="none" stroke={fill}
                    stroke-width="1.5" stroke-opacity="0.25"/>
            <circle cx={cx.to_string()} cy={cy.to_string()}
                    r={r.to_string()}
                    fill={fill} stroke="white" stroke-width="3"
                    filter={if is_sel { "url(#node-glow)" } else { "" }}/>
            <text x={cx.to_string()} y={(cy - 6.0).to_string()}
                  text-anchor="middle" dominant-baseline="central"
                  font-size="15" font-weight="800" fill="white">
                {score}
            </text>
            <text x={cx.to_string()} y={(cy + 10.0).to_string()}
                  text-anchor="middle" font-size="9" fill="rgba(255,255,255,0.85)">
                {label}
            </text>
            {if reason_count > 0 { html! {
                <>
                <circle cx={bx2.to_string()} cy={by2.to_string()}
                        r="10" fill="white" stroke={fill} stroke-width="1.5"/>
                <text x={bx2.to_string()} y={by2.to_string()}
                      text-anchor="middle" dominant-baseline="central"
                      font-size="9" font-weight="700" fill={fill}>
                    {reason_count.to_string()}
                </text>
                </>
            }} else { html! {} }}
        </g>
    }
}

fn node_color(tier: &str, score: u8) -> &'static str {
    match tier {
        "high" | "critical" => "#d8284f",
        "medium"            => "#b7791f",
        "low"               => "#1a7a3c",
        _ => if score >= 60 { "#d8284f" } else if score >= 30 { "#b7791f" } else { "#1a7a3c" },
    }
}

fn node_color_soft(tier: &str, score: u8) -> &'static str {
    match tier {
        "high" | "critical" => "#fff0f3",
        "medium"            => "#fff8e6",
        "low"               => "#edfaf2",
        _ => if score >= 60 { "#fff0f3" } else if score >= 30 { "#fff8e6" } else { "#edfaf2" },
    }
}

fn fmt_flag(flag: &str) -> String {
    match flag {
        "confirmed_fwa_history"             => "确认 FWA 历史".into(),
        "diagnosis_procedure_mismatch_rate" => "诊断/项目不匹配".into(),
        "high_cost_item_ratio"              => "高费项目占比高".into(),
        "peer_amount_p97"                   => "金额 P97 同行异常".into(),
        "peer_amount_p96"                   => "金额 P96 同行异常".into(),
        "peer_frequency_p96"                => "频率 P96 同行异常".into(),
        "peer_frequency_p97"                => "频率 P97 同行异常".into(),
        _                                   => flag.replace('_', " "),
    }
}

fn fmt_route(route: &str) -> &'static str {
    match route {
        "manual_review"     => "转人工审核",
        "provider_review"   => "Provider 专项审查",
        "siu_investigation" => "SIU 深度调查",
        "monitoring_only"   => "持续监控",
        "auto_flag"         => "自动标记",
        _                   => "人工评估",
    }
}

fn _use_display(v: &serde_json::Value) -> String {
    display_value(v)
}

// ── Shared helper used by other pages (model_ui_helpers, routing_policies, etc.)
// Preserved as pub(crate) to maintain cross-page compatibility
pub(crate) fn provider_signal_row(label: &str, value: &str, tone: &str) -> Html {
    let (bg, border) = match tone {
        "danger"  => ("var(--red-soft)",   "#d8284f"),
        "warning" => ("var(--amber-soft)", "#b7791f"),
        "success" | "strong" => ("#e8f7ee", "#1a7a3c"),
        _ => ("var(--surface-muted)", "var(--line-strong)"),
    };
    html! {
        <div style={format!(
            "display:flex;justify-content:space-between;align-items:center;
             padding:8px 11px;border-radius:7px;
             background:{bg};border-left:3px solid {border};margin-bottom:4px;"
        )}>
            <span style="font-size:12px;color:var(--muted);">{label}</span>
            <strong style="font-size:12px;color:var(--graphite);">{value}</strong>
        </div>
    }
}
