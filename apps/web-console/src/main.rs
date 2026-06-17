mod api;
mod case_helpers;
mod constants;
mod data_helpers;
mod data_lineage_helpers;
mod formatting;
mod i18n;
mod inbox_helpers;
mod medical_review_helpers;
mod model_ui_helpers;
mod ops_app;
mod ops_pages;
mod ops_routing;
mod pages;
mod payload_helpers;
mod routing;
mod rule_helpers;
mod rule_ui_helpers;
mod runtime_helpers;
mod state;
mod types;
mod ui_helpers;
mod visual_helpers;

use api::*;
use constants::*;
pub(crate) use data_helpers::*;
pub(crate) use formatting::*;
use i18n::setup_translations;
use ops_app::OpsApp;
use pages::*;
pub(crate) use rule_helpers::*;
use state::ApiState;
use types::*;
pub(crate) use ui_helpers::*;
pub(crate) use visual_helpers::*;

fn main() {
    setup_translations();
    yew::Renderer::<OpsApp>::new().render();
}
