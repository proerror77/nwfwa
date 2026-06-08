use crate::*;
use wasm_bindgen_futures::spawn_local;

#[function_component(MemberProfilePage)]
pub fn member_profile_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
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
                    Ok(profile) => ApiState::Ready(profile),
                    Err(error) => ApiState::Failed(error),
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
                    <h2>{"Member Profile"}</h2>
                    <p>{"Inspect the TPA-facing member profile summary used to explain utilization, policy exposure, high-risk history, and evidence-backed profile context."}</p>
                </div>
                <span class="status-pill">{"Profile Summary API"}</span>
            </div>

            <section class="panel">
                <h3>{"Member Profile Source"}</h3>
                <div class="form-grid">
                    {text_input("Member ID", &member_id)}
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*profile_state, ApiState::Loading)}>
                        {if matches!(&*profile_state, ApiState::Loading) { "Refreshing..." } else { "Refresh member profile" }}
                    </button>
                </div>
            </section>

            <MemberProfileView state={(*profile_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct MemberProfileProps {
    state: ApiState<MemberProfileSummary>,
}

#[function_component(MemberProfileView)]
fn member_profile_view(props: &MemberProfileProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Member Profile Summary"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Load a member profile summary to inspect utilization and evidence."}</p> },
                ApiState::Loading => html! { <p>{"Loading member profile..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(profile) => html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Member"}</span><strong>{&profile.member_id}</strong></div>
                            <div><span>{"Risk Summary"}</span><strong>{&profile.risk_level_summary}</strong></div>
                            <div><span>{"High-Risk Claims"}</span><strong>{profile.high_risk_claim_count}</strong></div>
                        </div>
                        {member_profile_cockpit(profile)}
                        <div class="summary-grid">
                            <div><span>{"Claims"}</span><strong>{profile.claim_count}</strong></div>
                            <div><span>{"Policies"}</span><strong>{profile.policy_count}</strong></div>
                            <div><span>{"Total Amount"}</span><strong>{format!("{} {}", display_value(&profile.total_claim_amount), profile.currency)}</strong></div>
                            <div><span>{"Latest Claim"}</span><strong>{profile.latest_claim_id.as_deref().unwrap_or("none")}</strong></div>
                            <div><span>{"Evidence Refs"}</span><strong>{profile.evidence_refs.len()}</strong></div>
                        </div>
                        <h4>{"Profile Narrative"}</h4>
                        <p>{&profile.profile_summary}</p>
                        <h4>{"Evidence"}</h4>
                        <small>{refs_label(&profile.evidence_refs)}</small>
                    </>
                },
            }}
        </section>
    }
}

fn member_profile_cockpit(profile: &MemberProfileSummary) -> Html {
    let total_amount = format!(
        "{} {}",
        display_value(&profile.total_claim_amount),
        profile.currency
    );
    html! {
        <div class="member-profile-cockpit">
            <div class="relationship-graph member-relationship-graph">
                <div class="graph-ring"></div>
                <div class="graph-ring inner"></div>
                <div class="graph-center member-profile-center">
                    <span>{"Member Evidence"}</span>
                    <strong>{&profile.member_id}</strong>
                </div>
                {member_graph_entity("Risk summary", &profile.risk_level_summary, "top", "lead")}
                {member_graph_entity("Claims", &profile.claim_count.to_string(), "right", "claim")}
                {member_graph_entity("Policy exposure", &profile.policy_count.to_string(), "bottom", "case")}
                {member_graph_entity("Latest claim", profile.latest_claim_id.as_deref().unwrap_or("none"), "left", "claim")}
                {member_graph_entity("Total amount", &total_amount, "lower-right", "provider")}
                {member_graph_entity("Evidence refs", &profile.evidence_refs.len().to_string(), "lower-left", "reviewer")}
            </div>
            <div class="member-evidence-panel">
                <div>
                    <span>{"Member Evidence Map"}</span>
                    <strong>{format!("{} high-risk / {} total claims", profile.high_risk_claim_count, profile.claim_count)}</strong>
                    <small>{&profile.profile_summary}</small>
                </div>
                <div class="member-signal-stack">
                    {member_signal_row("Utilization Snapshot", &format!("{} claims", profile.claim_count), "strong")}
                    {member_signal_row("Policy exposure", &format!("{} policies", profile.policy_count), "neutral")}
                    {member_signal_row("Risk amount", &total_amount, "warning")}
                    {member_signal_row("Evidence trace", &format!("{} refs", profile.evidence_refs.len()), "success")}
                </div>
                <small>{format!("evidence: {}", refs_label(&profile.evidence_refs))}</small>
            </div>
        </div>
    }
}

fn member_graph_entity(label: &str, value: &str, position: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("graph-entity", position.to_string(), tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn member_signal_row(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("member-signal-row", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}
