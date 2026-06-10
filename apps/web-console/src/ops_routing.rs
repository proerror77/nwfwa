#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OpsPage {
    ClaimsQueue,
    ReviewWorkbench,
    CaseTracker,
    RuleLibrary,
    Dashboard,
}

impl OpsPage {
    pub(crate) fn label(self) -> &'static str {
        match self {
            OpsPage::ClaimsQueue      => "理赔队列",
            OpsPage::ReviewWorkbench  => "审核工作台",
            OpsPage::CaseTracker      => "案件追踪",
            OpsPage::RuleLibrary      => "规则库",
            OpsPage::Dashboard        => "运营仪表盘",
        }
    }

    pub(crate) fn label_en(self) -> &'static str {
        match self {
            OpsPage::ClaimsQueue      => "Claims Queue",
            OpsPage::ReviewWorkbench  => "Review",
            OpsPage::CaseTracker      => "Cases",
            OpsPage::RuleLibrary      => "Rules",
            OpsPage::Dashboard        => "Dashboard",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            OpsPage::ClaimsQueue      => "claims",
            OpsPage::ReviewWorkbench  => "review",
            OpsPage::CaseTracker      => "cases",
            OpsPage::RuleLibrary      => "rules",
            OpsPage::Dashboard        => "dashboard",
        }
    }
}

pub(crate) const OPS_PAGES: &[OpsPage] = &[
    OpsPage::ClaimsQueue,
    OpsPage::ReviewWorkbench,
    OpsPage::CaseTracker,
    OpsPage::RuleLibrary,
    OpsPage::Dashboard,
];

pub(crate) fn ops_page_from_hash(hash: &str) -> OpsPage {
    let slug = hash.trim_start_matches('#');
    OPS_PAGES.iter().copied()
        .find(|p| p.slug() == slug)
        .unwrap_or(OpsPage::ClaimsQueue)
}

pub(crate) fn ops_set_hash(page: OpsPage) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(page.slug());
    }
}
