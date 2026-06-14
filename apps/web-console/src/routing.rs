use crate::i18n::tr;
use crate::state::Language;
use yew::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Module {
    IntakeOps,
    Dashboard,
    DiscoveryReview,
    RuntimeScoring,
    ReviewWorkbench,
    BootstrapOps,
    EvidenceHub,
    ProviderModelIntake,
    EvidenceRuntime,
    Rules,
    Models,
    RoutingPolicies,
    DataSources,
    FactorFactory,
    LeadsCases,
    MemberProfile,
    ProviderRisk,
    MedicalReview,
    AuditSampling,
    KnowledgeBase,
    AgentInvestigator,
    QaReview,
    Governance,
}

impl Module {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Module::IntakeOps => "Intake Ops",
            Module::Dashboard => "Dashboard",
            Module::DiscoveryReview => "Discovery Review",
            Module::RuntimeScoring => "Runtime Scoring",
            Module::ReviewWorkbench => "Review Workbench",
            Module::BootstrapOps => "Bootstrap Ops",
            Module::EvidenceHub => "Evidence Hub",
            Module::ProviderModelIntake => "Provider Model Intake",
            Module::EvidenceRuntime => "Evidence Runtime",
            Module::Rules => "Rules",
            Module::Models => "Models",
            Module::RoutingPolicies => "Routing Policies",
            Module::DataSources => "Data Sources",
            Module::FactorFactory => "Factor Factory",
            Module::LeadsCases => "Leads & Cases",
            Module::MemberProfile => "Member Profile",
            Module::ProviderRisk => "Provider Risk",
            Module::MedicalReview => "Medical Review",
            Module::AuditSampling => "Audit Sampling",
            Module::KnowledgeBase => "Knowledge Base",
            Module::AgentInvestigator => "Agent Investigator",
            Module::QaReview => "QA Review",
            Module::Governance => "Governance",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Module::IntakeOps => "intake-ops",
            Module::Dashboard => "dashboard",
            Module::DiscoveryReview => "discovery-review",
            Module::RuntimeScoring => "runtime-scoring",
            Module::ReviewWorkbench => "review-workbench",
            Module::BootstrapOps => "bootstrap-ops",
            Module::EvidenceHub => "evidence-hub",
            Module::ProviderModelIntake => "mlops-workspace",
            Module::EvidenceRuntime => "evidence-runtime",
            Module::Rules => "rules",
            Module::Models => "models",
            Module::RoutingPolicies => "routing-policies",
            Module::DataSources => "data-sources",
            Module::FactorFactory => "factor-factory",
            Module::LeadsCases => "leads-cases",
            Module::MemberProfile => "member-profile",
            Module::ProviderRisk => "provider-risk",
            Module::MedicalReview => "medical-review",
            Module::AuditSampling => "audit-sampling",
            Module::KnowledgeBase => "knowledge-base",
            Module::AgentInvestigator => "agent-investigator",
            Module::QaReview => "qa-review",
            Module::Governance => "governance",
        }
    }

    pub(crate) fn icon_class(self) -> &'static str {
        match self {
            Module::IntakeOps => "icon-inbox",
            Module::Dashboard => "icon-dashboard",
            Module::DiscoveryReview => "icon-routing",
            Module::RuntimeScoring => "icon-scoring",
            Module::ReviewWorkbench => "icon-qa",
            Module::BootstrapOps => "icon-audit",
            Module::EvidenceHub => "icon-knowledge",
            Module::ProviderModelIntake => "icon-models",
            Module::EvidenceRuntime => "icon-audit",
            Module::Rules => "icon-rules",
            Module::Models => "icon-models",
            Module::RoutingPolicies => "icon-routing",
            Module::FactorFactory => "icon-factors",
            Module::DataSources => "icon-data",
            Module::LeadsCases => "icon-cases",
            Module::MemberProfile => "icon-member",
            Module::ProviderRisk => "icon-provider",
            Module::MedicalReview => "icon-medical",
            Module::AuditSampling => "icon-audit",
            Module::KnowledgeBase => "icon-knowledge",
            Module::AgentInvestigator => "icon-agent",
            Module::QaReview => "icon-qa",
            Module::Governance => "icon-governance",
        }
    }
}

pub(crate) const DEFAULT_MODULE: Module = Module::Dashboard;

pub(crate) const NAV_SECTIONS: &[(&str, &[Module])] = &[
    (
        "Daily Ops",
        &[
            Module::Dashboard,
            Module::LeadsCases,
            Module::ReviewWorkbench,
        ],
    ),
    (
        "Intake & Scoring",
        &[Module::IntakeOps, Module::RuntimeScoring],
    ),
    (
        "Investigation",
        &[
            Module::EvidenceHub,
            Module::MemberProfile,
            Module::ProviderRisk,
            Module::KnowledgeBase,
            Module::AgentInvestigator,
            Module::AuditSampling,
        ],
    ),
    (
        "Governance & Tuning",
        &[
            Module::DiscoveryReview,
            Module::Rules,
            Module::Models,
            Module::RoutingPolicies,
            Module::ProviderModelIntake,
            Module::BootstrapOps,
            Module::DataSources,
            Module::FactorFactory,
            Module::MedicalReview,
            Module::QaReview,
            Module::EvidenceRuntime,
            Module::Governance,
        ],
    ),
];

pub(crate) const ALL_MODULES: &[Module] = &[
    Module::IntakeOps,
    Module::Dashboard,
    Module::DiscoveryReview,
    Module::RuntimeScoring,
    Module::ReviewWorkbench,
    Module::BootstrapOps,
    Module::EvidenceHub,
    Module::ProviderModelIntake,
    Module::EvidenceRuntime,
    Module::Rules,
    Module::Models,
    Module::RoutingPolicies,
    Module::DataSources,
    Module::FactorFactory,
    Module::LeadsCases,
    Module::MemberProfile,
    Module::ProviderRisk,
    Module::MedicalReview,
    Module::AuditSampling,
    Module::KnowledgeBase,
    Module::AgentInvestigator,
    Module::QaReview,
    Module::Governance,
];

pub(crate) fn workspace_system_map(
    active: Module,
    on_navigate: Callback<Module>,
    language: Language,
) -> Html {
    html! {
        <section class="workspace-system-map" aria-label="FWA platform system map">
            <div class="system-map-rail"></div>
            <div class="system-map-pulse"></div>
            {system_map_stage("Intake", tr(language, "Intake Ops", "进件处理"), tr(language, "TPA packet exceptions", "TPA 案件资料异常"), tr(language, "queue-ready claim", "可进入评分队列"), Module::IntakeOps, "intake", active, &on_navigate)}
            {system_map_stage("Detect", tr(language, "Scored leads", "已评分线索"), tr(language, "Rules + model + policy", "规则 + 模型 + 路由政策"), tr(language, "human queue", "人工队列"), Module::LeadsCases, "detect", active, &on_navigate)}
            {system_map_stage("Review", tr(language, "Human gate", "人工关卡"), tr(language, "Medical + QA", "医疗复核 + QA"), tr(language, "no auto denial", "不由模型自动拒赔"), Module::ReviewWorkbench, "review", active, &on_navigate)}
            {system_map_stage("Evidence", tr(language, "Case context", "案件上下文"), tr(language, "Member / provider / KB", "会员 / Provider / 知识库"), tr(language, "trace refs", "证据链"), Module::EvidenceHub, "evidence", active, &on_navigate)}
            {system_map_stage("Govern", tr(language, "Audit trail", "审计轨迹"), tr(language, "Policy + approval", "政策 + 审批"), tr(language, "pilot ready", "试点就绪"), Module::Governance, "govern", active, &on_navigate)}
            {system_map_stage("Value", tr(language, "Value proof", "价值证明"), tr(language, "Savings evidence", "节省金额证据"), tr(language, "dashboard", "仪表盘"), Module::Dashboard, "value", active, &on_navigate)}
        </section>
    }
}

pub(crate) fn active_module_from_location() -> Module {
    web_sys::window()
        .and_then(|window| window.location().hash().ok())
        .and_then(|hash| module_from_hash(&hash))
        .unwrap_or(DEFAULT_MODULE)
}

pub(crate) fn set_module_hash(module: Module) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let slug = module.slug();
    if window
        .location()
        .hash()
        .map(|hash| hash == format!("#{slug}"))
        .unwrap_or(false)
    {
        return;
    }
    let _ = window.location().set_hash(slug);
}

pub(crate) fn module_from_hash(hash: &str) -> Option<Module> {
    let slug = hash.trim_start_matches('#');
    if slug == "detection-releases" {
        return Some(Module::DiscoveryReview);
    }
    ALL_MODULES
        .iter()
        .copied()
        .find(|module| module.slug() == slug)
}

pub(crate) fn module_from_name(name: &str) -> Option<Module> {
    ALL_MODULES
        .iter()
        .copied()
        .find(|module| module.as_str() == name)
}

pub(crate) fn system_map_stage(
    step: &'static str,
    title: &'static str,
    detail: &'static str,
    outcome: &'static str,
    target: Module,
    tone: &'static str,
    active: Module,
    on_navigate: &Callback<Module>,
) -> Html {
    let on_navigate = on_navigate.clone();
    let is_active = active == target
        || (target == Module::ReviewWorkbench
            && matches!(active, Module::MedicalReview | Module::QaReview))
        || (target == Module::EvidenceHub
            && matches!(
                active,
                Module::EvidenceRuntime
                    | Module::ProviderRisk
                    | Module::MemberProfile
                    | Module::KnowledgeBase
                    | Module::DataSources
            ));
    html! {
        <button
            class={classes!("system-map-stage", tone, is_active.then_some("active"))}
            onclick={Callback::from(move |_| on_navigate.emit(target))}
        >
            <span class="system-stage-step">{step}</span>
            <span class="system-stage-glyph"></span>
            <strong>{title}</strong>
            <small>{detail}</small>
            <em>{outcome}</em>
        </button>
    }
}
