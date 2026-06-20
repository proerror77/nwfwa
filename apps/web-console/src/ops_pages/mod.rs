mod action_queue;
mod case_investigation;
mod case_tracker;
mod claims_queue;
mod investigate_workbench;
pub(crate) mod investigation_layers;
mod ops_dashboard;
mod system_learning;

pub(crate) use action_queue::ActionQueuePage;
// Kept for backward compat / internal references while the old pages still exist
pub(crate) use case_investigation::CaseInvestigationPage;
pub(crate) use case_tracker::CaseTrackerPage;
pub(crate) use claims_queue::ClaimsQueuePage;
pub(crate) use investigate_workbench::InvestigateWorkbenchPage;
pub(crate) use ops_dashboard::OpsDashboardPage;
pub(crate) use system_learning::SystemLearningPage;
