use crate::api::*;
use crate::types::*;
use crate::state::{use_api_key, ApiState};
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use serde_json::Value;

#[path = "rules_view.rs"]
mod rules_view;
use rules_view::RulesView;

#[path = "rules_discovery.rs"]
mod rules_discovery;
use rules_discovery::RulesDiscoveryWorkbench;

#[function_component(RulesPage)]
pub fn rules_page() -> Html {
    let api_key = use_api_key();
    let focused_rule_id = use_state(String::new);
    let snapshot_state = use_state(|| ApiState::<RuleOpsSnapshot>::Idle);
    let action_state = use_state(|| ApiState::<Value>::Idle);

    let load_rules = {
        let api_key = api_key.clone();
        let focused_rule_id = focused_rule_id.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_: ()| {
            let api_key = (*api_key).clone();
            let rule_id = (*focused_rule_id).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_rule_ops_snapshot(api_key, rule_id).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_rules = load_rules.clone();
        Callback::from(move |_| load_rules.emit(()))
    };

    {
        let load_rules = load_rules.clone();
        use_effect_with((), move |_| {
            load_rules.emit(());
            || ()
        });
    }

    let on_snapshot_refresh = {
        let load_rules = load_rules.clone();
        Callback::from(move |()| load_rules.emit(()))
    };

    let on_rule_saved = {
        let focused_rule_id = focused_rule_id.clone();
        let load_rules = load_rules.clone();
        Callback::from(move |saved_id: String| {
            focused_rule_id.set(saved_id);
            load_rules.emit(());
        })
    };

    // Derive selected rule status from snapshot for button guard logic.
    // The selected rule is identified by gates.rule_id (the focused rule).
    let selected_rule_status: Option<String> = match &*snapshot_state {
        ApiState::Ready(snapshot) => snapshot
            .rules
            .iter()
            .find(|r| r.rule_id == snapshot.gates.rule_id)
            .map(|r| r.status.clone()),
        _ => None,
    };

    let lifecycle_action = |action: &'static str| {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let action_state = action_state.clone();
        Callback::from(move |_: MouseEvent| {
            let api_key = (*api_key).clone();
            let rule_id = match &*snapshot_state {
                ApiState::Ready(snapshot) => snapshot.gates.rule_id.clone(),
                _ => return,
            };
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                action_state.set(
                    match post_rule_lifecycle(api_key, rule_id, action, vec![]).await {
                        Ok(val) => ApiState::Ready(val),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let loading = matches!(&*action_state, ApiState::Loading);
    let status_str = selected_rule_status.as_deref().unwrap_or("");

    // Disabled guards per backend state machine:
    //   submit   — always enabled (creates new candidate)
    //   approve  — requires status == "submitted"
    //   publish  — requires status == "approved"
    //   rollback — requires status == "active"
    let approve_disabled = loading || status_str != "submitted";
    let publish_disabled = loading || status_str != "approved";
    let rollback_disabled = loading || status_str != "active";

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"ML Rule Candidate Review"}</h2>
                    <p>{"Review rules discovered from model explanations, offline mining, or QA feedback. Operators run backtests, inspect shadow gates, and accept or reject the candidate before it can enter the governed rule library."}</p>
                </div>
                <span class="status-pill">{"Human review gate"}</span>
            </div>

            <RulesDiscoveryWorkbench
                api_key={(*api_key).clone()}
                on_snapshot_refresh={on_snapshot_refresh}
                on_rule_saved={on_rule_saved}
            />

            <div class="button-row">
                <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                    {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh gates" }}
                </button>
                <button onclick={lifecycle_action("submit")} disabled={loading}>
                    {"Submit"}
                </button>
                <button onclick={lifecycle_action("approve")} disabled={approve_disabled}>
                    {"Approve"}
                </button>
                <button onclick={lifecycle_action("publish")} disabled={publish_disabled}>
                    {"Publish"}
                </button>
                <button onclick={lifecycle_action("rollback")} disabled={rollback_disabled}>
                    {"Rollback"}
                </button>
            </div>

            {match &*action_state {
                ApiState::Idle => html! {},
                ApiState::Loading => html! { <p>{"Running rule lifecycle action..."}</p> },
                ApiState::Ready(_) => html! { <p class="success">{"Rule lifecycle action completed."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
            }}

            <RulesView state={(*snapshot_state).clone()} />
        </section>
    }
}
