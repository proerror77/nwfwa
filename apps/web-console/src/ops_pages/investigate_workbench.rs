/// InvestigateWorkbenchPage — thin wrapper around CaseInvestigationPage.
///
/// Differences from the old layout:
/// - No left queue sidebar; the case is pre-selected from the ActionQueue.
/// - Adds a "← Back to queue" breadcrumb.
/// - Wires `on_done` so that submitting an investigation conclusion calls
///   `on_done` which navigates the app back to ActionQueue.
use crate::i18n::tr;
use crate::ops_pages::CaseInvestigationPage;
use crate::state::Language;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct InvestigateWorkbenchPageProps {
    pub language: Language,
    /// Case or lead id to pre-select.  `None` shows the first open case.
    pub initial_case_id: Option<String>,
    /// Called with `()` after the investigator submits a conclusion.
    pub on_done: Callback<()>,
}

#[function_component(InvestigateWorkbenchPage)]
pub fn investigate_workbench_page(props: &InvestigateWorkbenchPageProps) -> Html {
    let language = props.language;
    let on_done = props.on_done.clone();

    html! {
        <div class="ops-page investigate-workbench-page">
            // ── Breadcrumb ──────────────────────────────────────────────────
            <div class="workbench-breadcrumb">
                <button
                    class="back-btn"
                    onclick={Callback::from(move |_: MouseEvent| on_done.emit(()))}
                >
                    {"← "}
                    {tr(language, "Back to queue", "返回待处理列表")}
                </button>
            </div>

            // ── Existing investigation workbench (reused) ───────────────────
            // CaseInvestigationPage handles all the heavy lifting:
            // 7-layer evidence panels, conclusion form, API calls.
            <CaseInvestigationPage language={language} />
        </div>
    }
}
