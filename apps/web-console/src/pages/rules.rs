use crate::api::*;
use crate::types::*;
use crate::constants::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::ui_helpers::*;
use crate::visual_helpers::*;
use crate::case_helpers::*;
use crate::rule_helpers::*;
use crate::rule_ui_helpers::*;
use crate::inbox_helpers::*;
use crate::payload_helpers::*;
use crate::data_helpers::*;
use crate::data_lineage_helpers::*;
use crate::medical_review_helpers::*;
use crate::model_ui_helpers::*;
use crate::runtime_helpers::*;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;

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
            </div>

            <RulesView state={(*snapshot_state).clone()} />
        </section>
    }
}
