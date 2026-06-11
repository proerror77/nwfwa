#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OpsPage {
    // ── 运营工作台 ────────────────────────────────────────────────────────────
    Dashboard,
    ClaimsQueue,
    ReviewWorkbench,
    CaseTracker,
    // ── 调查工具 ─────────────────────────────────────────────────────────────
    EvidenceHub,
    MemberProfile,
    ProviderRisk,
    KnowledgeBase,
    AgentInvestigator,
    // ── 规则与模型（推送接收）────────────────────────────────────────────────
    RuleLibrary,
    ModelGovernance,
    RoutingPolicies,
    // ── 质量管理 ─────────────────────────────────────────────────────────────
    GovernanceHub,
    AuditSampling,
    MedicalReview,
    QaReview,
}

impl OpsPage {
    pub(crate) fn label(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "运营仪表盘",
            OpsPage::ClaimsQueue => "理赔队列",
            OpsPage::ReviewWorkbench => "调查工作台",
            OpsPage::CaseTracker => "案件追踪",
            OpsPage::EvidenceHub => "证据中心",
            OpsPage::MemberProfile => "成员画像",
            OpsPage::ProviderRisk => "Provider 风险",
            OpsPage::KnowledgeBase => "知识库",
            OpsPage::AgentInvestigator => "AI 调查员",
            OpsPage::RuleLibrary => "规则库",
            OpsPage::ModelGovernance => "模型管理",
            OpsPage::RoutingPolicies => "路由策略",
            OpsPage::GovernanceHub => "质控与治理",
            OpsPage::AuditSampling => "抽样审核",
            OpsPage::MedicalReview => "医疗复核",
            OpsPage::QaReview => "QA 反馈",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "dashboard",
            OpsPage::ClaimsQueue => "claims",
            OpsPage::ReviewWorkbench => "review",
            OpsPage::CaseTracker => "cases",
            OpsPage::EvidenceHub => "evidence",
            OpsPage::MemberProfile => "member",
            OpsPage::ProviderRisk => "provider",
            OpsPage::KnowledgeBase => "knowledge",
            OpsPage::AgentInvestigator => "agent",
            OpsPage::RuleLibrary => "rules",
            OpsPage::ModelGovernance => "models",
            OpsPage::RoutingPolicies => "routing",
            OpsPage::GovernanceHub => "governance",
            OpsPage::AuditSampling => "audit",
            OpsPage::MedicalReview => "medical",
            OpsPage::QaReview => "qa",
        }
    }

    pub(crate) fn icon_class(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "icon-dashboard",
            OpsPage::ClaimsQueue => "icon-inbox",
            OpsPage::ReviewWorkbench => "icon-qa",
            OpsPage::CaseTracker => "icon-cases",
            OpsPage::EvidenceHub => "icon-knowledge",
            OpsPage::MemberProfile => "icon-member",
            OpsPage::ProviderRisk => "icon-provider",
            OpsPage::KnowledgeBase => "icon-knowledge",
            OpsPage::AgentInvestigator => "icon-agent",
            OpsPage::RuleLibrary => "icon-rules",
            OpsPage::ModelGovernance => "icon-models",
            OpsPage::RoutingPolicies => "icon-routing",
            OpsPage::GovernanceHub => "icon-governance",
            OpsPage::AuditSampling => "icon-audit",
            OpsPage::MedicalReview => "icon-medical",
            OpsPage::QaReview => "icon-qa",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "看今日风险、SLA 与队列负载",
            OpsPage::ClaimsQueue => "TPA 进件分流，不直接裁决",
            OpsPage::ReviewWorkbench => "人工调查证据、建议与补件",
            OpsPage::CaseTracker => "进行中案件 SLA 与进度",
            OpsPage::EvidenceHub => "证据链、画像、图谱与知识库",
            OpsPage::MemberProfile => "成员历史理赔与风险",
            OpsPage::ProviderRisk => "Provider 档案与图谱信号",
            OpsPage::KnowledgeBase => "相似案例与知识检索",
            OpsPage::AgentInvestigator => "AI 辅助调查包生成",
            OpsPage::RuleLibrary => "活跃规则 + 推送规则审核",
            OpsPage::ModelGovernance => "推送模型版本 + 激活决策",
            OpsPage::RoutingPolicies => "风险路由策略配置",
            OpsPage::GovernanceHub => "抽样、QA、医疗复核、规则模型治理",
            OpsPage::AuditSampling => "抽样质控与覆盖率",
            OpsPage::MedicalReview => "医疗必要性人工复核",
            OpsPage::QaReview => "QA 反馈闭环",
        }
    }
}

pub(crate) const PRIMARY_OPS_NAV: &[OpsPage] = &[
    OpsPage::Dashboard,
    OpsPage::ClaimsQueue,
    OpsPage::ReviewWorkbench,
    OpsPage::EvidenceHub,
    OpsPage::GovernanceHub,
];

pub(crate) const ALL_OPS_PAGES: &[OpsPage] = &[
    OpsPage::Dashboard,
    OpsPage::ClaimsQueue,
    OpsPage::ReviewWorkbench,
    OpsPage::CaseTracker,
    OpsPage::EvidenceHub,
    OpsPage::MemberProfile,
    OpsPage::ProviderRisk,
    OpsPage::KnowledgeBase,
    OpsPage::AgentInvestigator,
    OpsPage::RuleLibrary,
    OpsPage::ModelGovernance,
    OpsPage::RoutingPolicies,
    OpsPage::GovernanceHub,
    OpsPage::AuditSampling,
    OpsPage::MedicalReview,
    OpsPage::QaReview,
];

pub(crate) fn ops_page_from_hash(hash: &str) -> OpsPage {
    let slug = hash.trim_start_matches('#');
    ALL_OPS_PAGES
        .iter()
        .copied()
        .find(|p| p.slug() == slug)
        .unwrap_or(OpsPage::Dashboard)
}

pub(crate) fn ops_set_hash(page: OpsPage) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(page.slug());
    }
}
