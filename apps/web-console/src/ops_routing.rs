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
            OpsPage::ClaimsQueue     => "claims",
            OpsPage::ReviewWorkbench => "review",
            OpsPage::CaseTracker     => "cases",
            OpsPage::RuleLibrary     => "rules",
            OpsPage::Dashboard       => "dashboard",
        }
    }

    pub(crate) fn icon_class(self) -> &'static str {
        match self {
            OpsPage::ClaimsQueue     => "icon-inbox",
            OpsPage::ReviewWorkbench => "icon-qa",
            OpsPage::CaseTracker     => "icon-cases",
            OpsPage::RuleLibrary     => "icon-rules",
            OpsPage::Dashboard       => "icon-dashboard",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            OpsPage::ClaimsQueue     => "TPA 进件，风险判断与处置",
            OpsPage::ReviewWorkbench => "可疑案件调查与结论写回",
            OpsPage::CaseTracker     => "进行中案件 SLA 与状态",
            OpsPage::RuleLibrary     => "接受或拒绝推送的规则建议",
            OpsPage::Dashboard       => "今日拦截、防赔与运营指标",
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
