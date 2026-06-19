mod api;
mod case_helpers;
mod constants;
mod data_helpers;
mod data_lineage_helpers;
mod formatting;
mod i18n;
mod medical_review_helpers;
mod model_ui_helpers;
mod ops_app;
mod ops_pages;
mod ops_routing;
mod pages;
mod payload_helpers;
mod rule_helpers;
mod rule_ui_helpers;
mod state;
mod types;
mod ui_helpers;
mod visual_helpers;

pub(crate) use api::*;
pub(crate) use data_helpers::*;
pub(crate) use formatting::*;
use i18n::setup_translations;
use ops_app::OpsApp;
pub(crate) use pages::provider_signal_row;
pub(crate) use rule_helpers::*;
pub(crate) use state::ApiState;
pub(crate) use types::*;
pub(crate) use ui_helpers::*;

fn main() {
    setup_translations();
    yew::Renderer::<OpsApp>::new().render();
}
