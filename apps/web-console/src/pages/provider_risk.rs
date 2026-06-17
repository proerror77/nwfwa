/// Force-directed knowledge graph for FWA Provider-Member network
use crate::api::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use std::collections::HashMap;
use std::f64::consts::PI;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

// ─── Public entry point ───────────────────────────────────────────────────────

#[function_component(ProviderRiskPage)]
pub fn provider_risk_page() -> Html {
    let api_key = use_api_key();
    let data_state = use_state(|| ApiState::<GraphNetworkData>::Idle);
    let selected = use_state(|| Option::<String>::None);

    let load = {
        let api_key = api_key.clone();
        let data_state = data_state.clone();
        let selected = selected.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let data_state = data_state.clone();
            let selected = selected.clone();
            data_state.set(ApiState::Loading);
            spawn_local(async move {
                let result = get_graph_network_data(api_key).await;
                if let Ok(ref d) = result {
                    // Auto-select highest risk provider
                    if let Some(top) = d.providers.iter().max_by_key(|p| p.risk_score) {
                        selected.set(Some(top.provider_id.clone()));
                    }
                }
                data_state.set(match result {
                    Ok(d) => ApiState::Ready(d),
                    Err(e) => ApiState::Failed(e),
                });
            });
        })
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
                    <h2>{"Provider 风险网络图谱"}</h2>
                    <p>{"Force-directed 知识图谱：Provider 节点（大小=理赔量，颜色=风险）、Member 节点（蓝色）、连线=实际理赔关系。点击节点查看详情。"}</p>
                </div>
                <div style="display:flex;gap:8px;align-items:center;">
                    <span class="status-pill">{"FWA 网络分析"}</span>
                    <button onclick={Callback::from(move |_: MouseEvent| load.emit(()))} disabled={matches!(&*data_state, ApiState::Loading)}>
                        {if matches!(&*data_state, ApiState::Loading) { "加载中..." } else { "刷新图谱" }}
                    </button>
                </div>
            </div>

            {match &*data_state {
                ApiState::Idle => html! {
                    <section class="panel" style="padding:48px;text-align:center;">
                        <p class="empty">{"点击刷新加载 Provider 风险图谱"}</p>
                    </section>
                },
                ApiState::Loading => html! {
                    <section class="panel" style="padding:48px;text-align:center;">
                        <p class="empty">{"正在加载网络数据..."}</p>
                    </section>
                },
                ApiState::Failed(e) => html! {
                    <section class="panel"><p class="error">{e}</p></section>
                },
                ApiState::Ready(data) => html! {
                    <ForceGraph
                        data={data.clone()}
                        selected={(*selected).clone()}
                        on_select={{
                            let selected = selected.clone();
                            Callback::from(move |id: String| selected.set(Some(id)))
                        }}
                    />
                },
            }}
        </section>
    }
}

// ─── Graph node/edge types ────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum NodeKind {
    Provider,
    Member,
}

#[derive(Clone, Debug, PartialEq)]
struct GraphNode {
    id: String,
    label: String,
    kind: NodeKind,
    risk_score: u8,
    risk_tier: String,
    claim_count: u32,
    graph_reasons: Vec<String>,
    outlier_flags: Vec<String>,
    // layout position (mutable via use_state)
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct GraphEdge {
    source: String,
    target: String,
    strength: f64, // 0.0–1.0
    label: String,
    is_risk_link: bool, // shared outlier flag edge
}

// ─── Build graph from data ────────────────────────────────────────────────────

fn build_graph(data: &GraphNetworkData, w: f64, h: f64) -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let cx = w / 2.0;
    let cy = h / 2.0;

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

    // ── Semantic layout: sort providers by risk, place radially ──────────────
    // Highest risk → nearest center, lower risk → outer ring
    let mut sorted_providers = data.providers.clone();
    sorted_providers.sort_by(|a, b| b.risk_score.cmp(&a.risk_score));

    let n = sorted_providers.len();
    for (rank, p) in sorted_providers.iter().enumerate() {
        // radius increases with rank (lower risk = further from center)
        let base_r = if rank == 0 {
            0.0 // highest risk node at center
        } else {
            let tier = (rank as f64 / n.max(2) as f64).sqrt();
            w.min(h) * 0.22 + tier * w.min(h) * 0.28
        };

        // Spread nodes evenly around the ring at each tier
        // All nodes at the same radius get evenly spaced angles
        let nodes_at_ring: usize = if rank == 0 { 1 } else { n - 1 };
        let ring_idx = if rank == 0 { 0 } else { rank - 1 };
        let angle = if rank == 0 {
            0.0
        } else {
            // Start from top (−π/2) and go clockwise
            -PI / 2.0 + 2.0 * PI * ring_idx as f64 / nodes_at_ring as f64
        };

        nodes.push(GraphNode {
            id: p.provider_id.clone(),
            label: short_id(&p.provider_id),
            kind: NodeKind::Provider,
            risk_score: p.risk_score,
            risk_tier: p.risk_tier.clone(),
            claim_count: p.claim_count,
            graph_reasons: p.graph_reasons.clone(),
            outlier_flags: p.outlier_flags.clone(),
            x: cx + base_r * angle.cos(),
            y: cy + base_r * angle.sin(),
            vx: 0.0,
            vy: 0.0,
        });
    }

    // ── Member nodes: place near their primary provider ───────────────────────
    // Map member → provider with highest-risk lead
    let mut member_primary: HashMap<String, (String, u8)> = HashMap::new();
    for lead in &data.leads {
        let entry = member_primary
            .entry(lead.member_id.clone())
            .or_insert((lead.provider_id.clone(), lead.risk_score));
        if lead.risk_score > entry.1 {
            *entry = (lead.provider_id.clone(), lead.risk_score);
        }
    }

    // Count members per provider to space them out
    let mut provider_member_count: HashMap<String, usize> = HashMap::new();
    for (_, (pid, _)) in &member_primary {
        *provider_member_count.entry(pid.clone()).or_insert(0) += 1;
    }
    let mut provider_member_idx: HashMap<String, usize> = HashMap::new();

    let mut seen_members: HashMap<String, u32> = HashMap::new();
    for lead in &data.leads {
        *seen_members.entry(lead.member_id.clone()).or_insert(0) += 1;
    }

    for (member_id, count) in &seen_members {
        let (primary_pid, _) = member_primary.get(member_id).cloned().unwrap_or_default();

        // Find provider node position
        let (px, py) = nodes
            .iter()
            .find(|n| n.id == primary_pid)
            .map(|n| (n.x, n.y))
            .unwrap_or((cx, cy));

        // Spread members around their provider at a fixed offset radius
        let total = provider_member_count
            .get(&primary_pid)
            .copied()
            .unwrap_or(1);
        let idx = *provider_member_idx.entry(primary_pid.clone()).or_insert(0);
        *provider_member_idx.get_mut(&primary_pid).unwrap() += 1;

        let spread_angle = 2.0 * PI * idx as f64 / total.max(1) as f64;
        // Direction away from center so member doesn't overlap provider
        let dir_x = px - cx;
        let dir_y = py - cy;
        let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(1.0);
        // Base offset: 52px outward from provider, then fan around
        let offset_r = 52.0;
        let fan_angle = spread_angle * 0.6 - 0.3; // ±0.3 rad fan
        let ox = (dir_x / dir_len) * offset_r * fan_angle.cos()
            - (dir_y / dir_len) * offset_r * fan_angle.sin();
        let oy = (dir_x / dir_len) * offset_r * fan_angle.sin()
            + (dir_y / dir_len) * offset_r * fan_angle.cos();

        nodes.push(GraphNode {
            id: member_id.clone(),
            label: short_id(member_id),
            kind: NodeKind::Member,
            risk_score: 0,
            risk_tier: "member".into(),
            claim_count: *count,
            graph_reasons: Vec::new(),
            outlier_flags: Vec::new(),
            x: (px + ox).clamp(30.0, w - 30.0),
            y: (py + oy).clamp(30.0, h - 30.0),
            vx: 0.0,
            vy: 0.0,
        });
    }

    // ── Edges ─────────────────────────────────────────────────────────────────

    // Provider ↔ Member edges (deduplicated: one edge per unique pair)
    let mut pm_pairs: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    for lead in &data.leads {
        let key = (lead.provider_id.clone(), lead.member_id.clone());
        if pm_pairs.insert(key) {
            let strength = (lead.risk_score as f64 / 100.0).max(0.2);
            edges.push(GraphEdge {
                source: lead.provider_id.clone(),
                target: lead.member_id.clone(),
                strength,
                label: format!("理赔 {}", lead.claim_id),
                is_risk_link: lead.risk_score >= 60,
            });
        }
    }

    // Provider ↔ Provider edges: shared outlier flags
    for i in 0..sorted_providers.len() {
        for j in (i + 1)..sorted_providers.len() {
            let pa = &sorted_providers[i];
            let pb = &sorted_providers[j];
            let shared: Vec<_> = pa
                .outlier_flags
                .iter()
                .filter(|f| pb.outlier_flags.contains(f))
                .collect();
            if !shared.is_empty() {
                edges.push(GraphEdge {
                    source: pa.provider_id.clone(),
                    target: pb.provider_id.clone(),
                    strength: shared.len() as f64 / 5.0,
                    label: format!("共同异常: {}", shared.len()),
                    is_risk_link: true,
                });
            }
        }
    }

    (nodes, edges)
}

// Deterministic jitter from seed (kept for potential future use)
#[allow(dead_code)]
fn rand_jitter(seed: f64) -> f64 {
    let x = (seed * 127.1 + 311.7).sin() * 43758.5453;
    (x - x.floor() - 0.5) * 60.0
}

fn short_id(id: &str) -> String {
    if id.chars().count() > 12 {
        format!("{}…", id.chars().take(12).collect::<String>())
    } else {
        id.to_string()
    }
}

// ─── Main graph component ─────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct ForceGraphProps {
    data: GraphNetworkData,
    selected: Option<String>,
    on_select: Callback<String>,
}

#[function_component(ForceGraph)]
fn force_graph(props: &ForceGraphProps) -> Html {
    let w = 720.0_f64;
    let h = 520.0_f64;

    // Layout is pre-computed synchronously in build_graph (120 FR iterations).
    // We only re-compute when the underlying data changes (keyed by provider count).
    let data_key = props.data.providers.len();
    let (init_nodes, edges) = build_graph(&props.data, w, h);
    let nodes_state = use_state(|| init_nodes.clone());

    {
        let nodes_state = nodes_state.clone();
        let data = props.data.clone();
        use_effect_with(data_key, move |_| {
            let (nodes, _) = build_graph(&data, w, h);
            nodes_state.set(nodes);
            || ()
        });
    }

    let nodes = &*nodes_state;
    let selected_id = props.selected.as_deref();

    // Find selected node for detail panel
    let selected_node = selected_id.and_then(|id| nodes.iter().find(|n| n.id == id));
    let selected_provider = selected_node.and_then(|n| {
        if matches!(n.kind, NodeKind::Provider) {
            props.data.providers.iter().find(|p| p.provider_id == n.id)
        } else {
            None
        }
    });

    let max_claims = nodes
        .iter()
        .map(|n| n.claim_count)
        .max()
        .unwrap_or(1)
        .max(1);

    html! {
        <div style="display:grid;grid-template-columns:1fr 360px;gap:14px;align-items:start;">
            // ── Graph canvas ──────────────────────────────────────────────────
            <div>
                // Stats strip
                <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:10px;margin-bottom:12px;">
                    {stat_chip(&format!("{} Providers", props.data.providers.len()), "#d8284f")}
                    {stat_chip(&format!("{} Members", nodes.iter().filter(|n| matches!(n.kind, NodeKind::Member)).count().to_string() + " 成员"), "#1769e0")}
                    {stat_chip(&format!("{} 理赔连接", props.data.leads.len()), "#0f7b8c")}
                    {stat_chip("布局完成 ✓", "#5f6f85")}
                </div>

                // Dark canvas
                <div style="border-radius:12px;overflow:hidden;box-shadow:0 4px 24px rgba(5,38,73,0.3);">
                    <svg
                        viewBox={format!("0 0 {w} {h}")}
                        style="width:100%;height:520px;display:block;background:#0d1117;"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <defs>
                            <filter id="glow-red">
                                <feGaussianBlur stdDeviation="4" result="blur"/>
                                <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
                            </filter>
                            <filter id="glow-blue">
                                <feGaussianBlur stdDeviation="3" result="blur"/>
                                <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
                            </filter>
                            // Subtle grid
                            <pattern id="dark-grid" width="40" height="40" patternUnits="userSpaceOnUse">
                                <path d="M 40 0 L 0 0 0 40" fill="none"
                                      stroke="rgba(255,255,255,0.03)" stroke-width="0.5"/>
                            </pattern>
                        </defs>
                        <rect width={w.to_string()} height={h.to_string()} fill="url(#dark-grid)"/>

                        // ── Edges ──────────────────────────────────────────────
                        {for edges.iter().map(|edge| {
                            let s_pos = nodes.iter().find(|n| n.id == edge.source)
                                .map(|n| (n.x, n.y));
                            let t_pos = nodes.iter().find(|n| n.id == edge.target)
                                .map(|n| (n.x, n.y));
                            if let (Some((sx, sy)), Some((tx, ty))) = (s_pos, t_pos) {
                                let opacity = 0.15 + edge.strength * 0.35;
                                let (stroke, dash) = if edge.is_risk_link {
                                    ("#ff4d6d", "")
                                } else {
                                    ("#4c9be8", "4,4")
                                };
                                let sw = 0.5 + edge.strength * 1.5;
                                html! {
                                    <line
                                        x1={sx.to_string()} y1={sy.to_string()}
                                        x2={tx.to_string()} y2={ty.to_string()}
                                        stroke={stroke}
                                        stroke-width={format!("{sw:.1}")}
                                        stroke-opacity={format!("{opacity:.2}")}
                                        stroke-dasharray={dash}
                                    />
                                }
                            } else {
                                html! {}
                            }
                        })}

                        // ── Nodes ──────────────────────────────────────────────
                        {for nodes.iter().map(|node| {
                            let is_sel = selected_id == Some(&node.id);
                            let on_select = props.on_select.clone();
                            let nid = node.id.clone();

                            match node.kind {
                                NodeKind::Provider => {
                                    let base_r = 10.0 + 22.0 * (node.claim_count as f64 / max_claims as f64).sqrt();
                                    let r = if is_sel { base_r + 4.0 } else { base_r };
                                    let (fill, glow_id) = match node.risk_tier.as_str() {
                                        "high" | "critical" => ("#ff4d6d", "glow-red"),
                                        "medium" => ("#f59e0b", "glow-red"),
                                        _ => ("#34d399", "glow-blue"),
                                    };
                                    let fill_dim = match node.risk_tier.as_str() {
                                        "high" | "critical" => "rgba(255,77,109,0.15)",
                                        "medium" => "rgba(245,158,11,0.15)",
                                        _ => "rgba(52,211,153,0.15)",
                                    };

                                    html! {
                                        <g style="cursor:pointer;"
                                           onclick={Callback::from(move |_| on_select.emit(nid.clone()))}>
                                            // Halo
                                            if is_sel {
                                                <circle cx={node.x.to_string()} cy={node.y.to_string()}
                                                        r={(r + 10.0).to_string()}
                                                        fill={fill_dim}
                                                        stroke={fill}
                                                        stroke-width="1"
                                                        stroke-opacity="0.4"/>
                                            }
                                            // Node
                                            <circle cx={node.x.to_string()} cy={node.y.to_string()}
                                                    r={r.to_string()}
                                                    fill={fill_dim}
                                                    stroke={fill}
                                                    stroke-width={if is_sel { "2.5" } else { "1.5" }}
                                                    filter={if is_sel { glow_id } else { "" }}/>
                                            // Score label inside
                                            <text x={node.x.to_string()} y={node.y.to_string()}
                                                  text-anchor="middle" dominant-baseline="central"
                                                  font-size={if r > 16.0 { "11" } else { "9" }}
                                                  font-weight="700" fill={fill}>
                                                {node.risk_score.to_string()}
                                            </text>
                                            // Name label below
                                            <text x={node.x.to_string()} y={(node.y + r + 12.0).to_string()}
                                                  text-anchor="middle"
                                                  font-size="9" fill="rgba(200,210,230,0.7)">
                                                {&node.label}
                                            </text>
                                            // Outlier count badge
                                            if !node.outlier_flags.is_empty() {
                                                <circle cx={(node.x + r * 0.72).to_string()}
                                                        cy={(node.y - r * 0.72).to_string()}
                                                        r="7" fill="#ff4d6d" stroke="#0d1117" stroke-width="1.5"/>
                                                <text x={(node.x + r * 0.72).to_string()}
                                                      y={(node.y - r * 0.72).to_string()}
                                                      text-anchor="middle" dominant-baseline="central"
                                                      font-size="7" font-weight="700" fill="white">
                                                    {node.outlier_flags.len().to_string()}
                                                </text>
                                            }
                                        </g>
                                    }
                                }
                                NodeKind::Member => {
                                    let r = if is_sel { 9.0 } else { 6.0 };
                                    html! {
                                        <g style="cursor:pointer;"
                                           onclick={Callback::from(move |_| on_select.emit(nid.clone()))}>
                                            <circle cx={node.x.to_string()} cy={node.y.to_string()}
                                                    r={r.to_string()}
                                                    fill="rgba(71,153,232,0.2)"
                                                    stroke={if is_sel { "#60a5fa" } else { "#4c9be8" }}
                                                    stroke-width={if is_sel { "2" } else { "1" }}
                                                    filter={if is_sel { "url(#glow-blue)" } else { "" }}/>
                                            <text x={node.x.to_string()} y={(node.y + r + 10.0).to_string()}
                                                  text-anchor="middle"
                                                  font-size="8" fill="rgba(147,186,234,0.65)">
                                                {&node.label}
                                            </text>
                                        </g>
                                    }
                                }
                            }
                        })}

                        // ── Legend ─────────────────────────────────────────────
                        <g transform="translate(12, 490)">
                            <circle cx="8" cy="7" r="5" fill="rgba(255,77,109,0.2)" stroke="#ff4d6d" stroke-width="1.5"/>
                            <text x="18" y="11" font-size="9" fill="rgba(200,210,230,0.6)">{"高风险 Provider"}</text>
                            <circle cx="108" cy="7" r="5" fill="rgba(245,158,11,0.2)" stroke="#f59e0b" stroke-width="1.5"/>
                            <text x="118" y="11" font-size="9" fill="rgba(200,210,230,0.6)">{"中风险"}</text>
                            <circle cx="168" cy="7" r="4" fill="rgba(71,153,232,0.2)" stroke="#4c9be8" stroke-width="1"/>
                            <text x="178" y="11" font-size="9" fill="rgba(200,210,230,0.6)">{"Member"}</text>
                            <line x1="240" y1="7" x2="260" y2="7" stroke="#ff4d6d" stroke-width="1.5" stroke-opacity="0.6"/>
                            <text x="265" y="11" font-size="9" fill="rgba(200,210,230,0.6)">{"风险连接"}</text>
                            <line x1="330" y1="7" x2="350" y2="7" stroke="#4c9be8" stroke-width="1" stroke-dasharray="3,3" stroke-opacity="0.6"/>
                            <text x="355" y="11" font-size="9" fill="rgba(200,210,230,0.6)">{"理赔连接"}</text>
                            <text x="430" y="11" font-size="9" fill="rgba(200,210,230,0.4)">{"节点大小 = 理赔量　数字 = 风险分"}</text>
                        </g>
                    </svg>
                </div>
            </div>

            // ── Detail panel ───────────────────────────────────────────────────
            <div style="position:sticky;top:16px;">
                {if let Some(provider) = selected_provider {
                    html! { <ProviderDetailPanel provider={provider.clone()} /> }
                } else if let Some(node) = selected_node {
                    // Member detail
                    member_detail_panel(node, &props.data.leads)
                } else {
                    html! {
                        <section class="panel" style="background:#161b22;border:1px solid #30363d;padding:40px 20px;text-align:center;">
                            <p style="color:rgba(200,210,230,0.4);font-size:13px;">
                                {"点击图谱中的节点查看详情"}
                            </p>
                        </section>
                    }
                }}
            </div>
        </div>
    }
}

// ─── Detail panels ────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct DetailPanelProps {
    provider: ProviderRiskItem,
}

#[function_component(ProviderDetailPanel)]
fn provider_detail_panel(props: &DetailPanelProps) -> Html {
    let p = &props.provider;
    let fill = provider_color(&p.risk_tier, p.risk_score);

    html! {
        <section style="background:#161b22;border:1px solid #30363d;border-radius:10px;overflow:hidden;display:flex;flex-direction:column;gap:0;">
            // Header
            <div style={format!(
                "padding:14px 16px;border-bottom:1px solid #30363d;
                 display:flex;align-items:center;gap:10px;
                 background:rgba({},0.08);", rgb_from_hex(fill)
            )}>
                <div style={format!(
                    "width:40px;height:40px;border-radius:50%;flex-shrink:0;
                     border:2px solid {fill};background:rgba({},0.15);
                     display:flex;align-items:center;justify-content:center;
                     font-size:14px;font-weight:800;color:{fill};",
                    rgb_from_hex(fill)
                )}>
                    {p.risk_score.to_string()}
                </div>
                <div style="min-width:0;">
                    <div style="font-size:13px;font-weight:700;color:#e6edf3;overflow-wrap:anywhere;">{&p.provider_id}</div>
                    <div style="font-size:11px;color:#8b949e;margin-top:2px;">
                        {p.specialty.as_deref().unwrap_or("未知专科")}
                        {"  ·  "}
                        {format!("{} 笔理赔", p.claim_count)}
                    </div>
                </div>
                if p.review_required {
                    <span style="margin-left:auto;padding:2px 7px;background:#ff4d6d;
                                 color:white;border-radius:4px;font-size:10px;font-weight:700;
                                 flex-shrink:0;">{"需审核"}</span>
                }
            </div>

            <div style="padding:14px 16px;display:flex;flex-direction:column;gap:12px;">
                // Score bars
                {dark_score_bar("综合风险评分", p.risk_score, fill)}
                if let Some(net) = p.network_risk_score {
                    {dark_score_bar("图谱网络风险", net, fill)}
                }

                // Stats
                <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:6px;">
                    {dark_stat("理赔量", &p.claim_count.to_string())}
                    {dark_stat("确认 FWA", &p.confirmed_fwa_count.to_string())}
                    {dark_stat("误报", &p.false_positive_count.to_string())}
                </div>

                // Graph reasons
                if !p.graph_reasons.is_empty() {
                    <div>
                        <div style="font-size:10px;font-weight:700;text-transform:uppercase;
                                    letter-spacing:.07em;color:#8b949e;margin-bottom:7px;">
                            {format!("图谱风险原因 ({})", p.graph_reasons.len())}
                        </div>
                        {for p.graph_reasons.iter().map(|r| html! {
                            <div style="display:flex;gap:8px;padding:8px 10px;border-radius:6px;
                                        background:rgba(255,77,109,0.07);border-left:2px solid #ff4d6d;
                                        margin-bottom:5px;align-items:flex-start;">
                                <span style="color:#ff4d6d;font-size:12px;flex-shrink:0;">{"⚠"}</span>
                                <span style="font-size:11px;line-height:1.5;color:#cdd9e5;">{r}</span>
                            </div>
                        })}
                    </div>
                }

                // Outlier flags
                if !p.outlier_flags.is_empty() {
                    <div>
                        <div style="font-size:10px;font-weight:700;text-transform:uppercase;
                                    letter-spacing:.07em;color:#8b949e;margin-bottom:7px;">
                            {"异常标记"}
                        </div>
                        <div style="display:flex;flex-wrap:wrap;gap:5px;">
                            {for p.outlier_flags.iter().map(|flag| html! {
                                <span style="padding:3px 8px;border-radius:4px;font-size:10px;font-weight:600;
                                              background:rgba(245,158,11,0.1);color:#f59e0b;
                                              border:1px solid rgba(245,158,11,0.3);">
                                    {fmt_flag(flag)}
                                </span>
                            })}
                        </div>
                    </div>
                }
            </div>
        </section>
    }
}

fn member_detail_panel(node: &GraphNode, leads: &[LeadRecord]) -> Html {
    let member_leads: Vec<_> = leads.iter().filter(|l| l.member_id == node.id).collect();

    html! {
        <section style="background:#161b22;border:1px solid #30363d;border-radius:10px;overflow:hidden;">
            <div style="padding:14px 16px;border-bottom:1px solid #30363d;
                        background:rgba(71,153,232,0.06);
                        display:flex;align-items:center;gap:10px;">
                <div style="width:36px;height:36px;border-radius:50%;flex-shrink:0;
                             border:2px solid #4c9be8;background:rgba(76,155,232,0.12);
                             display:flex;align-items:center;justify-content:center;
                             font-size:13px;color:#60a5fa;">
                    {"M"}
                </div>
                <div>
                    <div style="font-size:13px;font-weight:700;color:#e6edf3;">{&node.id}</div>
                    <div style="font-size:11px;color:#8b949e;">{format!("{} 笔理赔记录", member_leads.len())}</div>
                </div>
            </div>
            <div style="padding:12px 16px;display:flex;flex-direction:column;gap:8px;">
                <div style="font-size:10px;font-weight:700;text-transform:uppercase;
                            letter-spacing:.07em;color:#8b949e;margin-bottom:2px;">
                    {"关联理赔"}
                </div>
                {for member_leads.iter().map(|lead| html! {
                    <div style="padding:8px 10px;border-radius:6px;background:#0d1117;
                                border:1px solid #21262d;display:flex;justify-content:space-between;
                                align-items:center;">
                        <div>
                            <div style="font-size:11px;font-weight:600;color:#e6edf3;">{&lead.claim_id}</div>
                            <div style="font-size:10px;color:#8b949e;">{&lead.provider_id}</div>
                        </div>
                        <span style={format!(
                            "padding:2px 7px;border-radius:4px;font-size:10px;font-weight:700;{}",
                            if lead.risk_score >= 70 { "background:rgba(255,77,109,0.15);color:#ff4d6d;" }
                            else if lead.risk_score >= 40 { "background:rgba(245,158,11,0.15);color:#f59e0b;" }
                            else { "background:rgba(52,211,153,0.15);color:#34d399;" }
                        )}>
                            {lead.risk_score.to_string()}
                        </span>
                    </div>
                })}
            </div>
        </section>
    }
}

// ─── Small helpers ────────────────────────────────────────────────────────────

fn stat_chip(label: &str, color: &str) -> Html {
    html! {
        <div style={format!(
            "padding:8px 12px;border-radius:7px;text-align:center;
             background:rgba({},0.08);border:1px solid rgba({},0.2);",
            rgb_from_hex(color), rgb_from_hex(color)
        )}>
            <span style={format!("font-size:12px;font-weight:600;color:{color};")}>{label}</span>
        </div>
    }
}

fn dark_score_bar(label: &str, score: u8, fill: &str) -> Html {
    html! {
        <div>
            <div style="display:flex;justify-content:space-between;font-size:11px;margin-bottom:4px;">
                <span style="color:#8b949e;">{label}</span>
                <span style={format!("font-weight:700;color:{fill};")}>{format!("{score}")}</span>
            </div>
            <div style="height:5px;background:#21262d;border-radius:3px;overflow:hidden;">
                <div style={format!(
                    "height:100%;background:{fill};border-radius:3px;
                     width:{score}%;transition:width .5s;"
                )}></div>
            </div>
        </div>
    }
}

fn dark_stat(label: &str, value: &str) -> Html {
    html! {
        <div style="text-align:center;padding:8px 6px;background:#0d1117;border-radius:6px;border:1px solid #21262d;">
            <div style="font-size:16px;font-weight:800;color:#e6edf3;">{value}</div>
            <div style="font-size:9px;color:#8b949e;margin-top:2px;">{label}</div>
        </div>
    }
}

fn provider_color(tier: &str, score: u8) -> &'static str {
    match tier {
        "high" | "critical" => "#ff4d6d",
        "medium" => "#f59e0b",
        "low" => "#34d399",
        _ => {
            if score >= 60 {
                "#ff4d6d"
            } else if score >= 30 {
                "#f59e0b"
            } else {
                "#34d399"
            }
        }
    }
}

fn rgb_from_hex(hex: &str) -> &'static str {
    match hex {
        "#ff4d6d" => "255,77,109",
        "#f59e0b" => "245,158,11",
        "#34d399" => "52,211,153",
        "#4c9be8" => "76,155,232",
        _ => "100,100,100",
    }
}

fn fmt_flag(flag: &str) -> String {
    match flag {
        "confirmed_fwa_history" => "确认 FWA 历史".into(),
        "diagnosis_procedure_mismatch_rate" => "诊断/项目不匹配".into(),
        "high_cost_item_ratio" => "高费项目占比".into(),
        "peer_amount_p97" => "金额 P97 异常".into(),
        "peer_amount_p96" => "金额 P96 异常".into(),
        "peer_frequency_p96" => "频率 P96 异常".into(),
        "peer_frequency_p97" => "频率 P97 异常".into(),
        _ => flag.replace('_', " "),
    }
}

// Keep for compatibility with other pages (routing_policies, model_ui_helpers, etc.)
pub(crate) fn provider_signal_row(label: &str, value: &str, tone: &str) -> Html {
    let (bg, border) = match tone {
        "danger" => ("var(--red-soft)", "#d8284f"),
        "warning" => ("var(--amber-soft)", "#b7791f"),
        "success" | "strong" => ("#e8f7ee", "#1a7a3c"),
        _ => ("var(--surface-muted)", "var(--line-strong)"),
    };
    html! {
        <div style={format!(
            "display:flex;justify-content:space-between;align-items:center;
             padding:8px 11px;border-radius:7px;background:{bg};
             border-left:3px solid {border};margin-bottom:4px;"
        )}>
            <span style="font-size:12px;color:var(--muted);">{label}</span>
            <strong style="font-size:12px;color:var(--graphite);">{value}</strong>
        </div>
    }
}
