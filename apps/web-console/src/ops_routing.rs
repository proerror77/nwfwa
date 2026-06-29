use crate::state::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OpsPage {
    /// Default home: what needs review today and why it matters.
    Dashboard,
    /// Items needing human action: investigate / pending-evidence / medical.
    ActionQueue,
    /// Single-case deep-dive workbench entered from ActionQueue.
    Investigate,
    /// Governance loop: reviewer feedback, rules, model drift, QA.
    SystemLearning,
}

impl OpsPage {
    pub(crate) fn label(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "今日概况",
            OpsPage::ActionQueue => "需要处理",
            OpsPage::Investigate => "调查工作台",
            OpsPage::SystemLearning => "治理闭环",
        }
    }

    pub(crate) fn label_en(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "Today",
            OpsPage::ActionQueue => "Work Queue",
            OpsPage::Investigate => "Case Workbench",
            OpsPage::SystemLearning => "Governance Loop",
        }
    }

    pub(crate) fn label_for(self, language: Language) -> &'static str {
        match language {
            Language::En => self.label_en(),
            Language::Zh => self.label(),
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "dashboard",
            OpsPage::ActionQueue => "queue",
            OpsPage::Investigate => "investigate",
            OpsPage::SystemLearning => "governance",
        }
    }

    pub(crate) fn icon_class(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "icon-dashboard",
            OpsPage::ActionQueue => "icon-inbox",
            OpsPage::Investigate => "icon-qa",
            OpsPage::SystemLearning => "icon-governance",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "今天有哪些风险线索需要人处理",
            OpsPage::ActionQueue => "按下一步动作整理线索和案件",
            OpsPage::Investigate => "事实、证据、建议和审计同屏处理",
            OpsPage::SystemLearning => "把复核结论回流到规则、模型和质检",
        }
    }

    pub(crate) fn description_en(self) -> &'static str {
        match self {
            OpsPage::Dashboard => "What needs human review today",
            OpsPage::ActionQueue => "Leads and cases grouped by next action",
            OpsPage::Investigate => "Facts, evidence, recommendation, and audit trail together",
            OpsPage::SystemLearning => "Reviewer outcomes feed rules, models, and QA",
        }
    }

    pub(crate) fn description_for(self, language: Language) -> &'static str {
        match language {
            Language::En => self.description_en(),
            Language::Zh => self.description(),
        }
    }

    /// Resolve a page by its English label string.
    pub(crate) fn from_label(label: &str) -> Option<Self> {
        ALL_OPS_PAGES
            .iter()
            .copied()
            .find(|p| p.label_en() == label)
    }
}

pub(crate) struct OpsNavGroup {
    title_en: &'static str,
    title_zh: &'static str,
    pages: &'static [OpsPage],
}

impl OpsNavGroup {
    pub(crate) fn title_for(&self, language: Language) -> &'static str {
        match language {
            Language::En => self.title_en,
            Language::Zh => self.title_zh,
        }
    }

    pub(crate) fn pages(&self) -> &'static [OpsPage] {
        self.pages
    }
}

pub(crate) const ALL_OPS_PAGES: &[OpsPage] = &[
    OpsPage::Dashboard,
    OpsPage::ActionQueue,
    OpsPage::Investigate,
    OpsPage::SystemLearning,
];

pub(crate) const OPS_NAV_GROUPS: &[OpsNavGroup] = &[OpsNavGroup {
    title_en: "",
    title_zh: "",
    pages: ALL_OPS_PAGES,
}];

pub(crate) fn ops_page_from_hash(hash: &str) -> OpsPage {
    let slug = hash.trim_start_matches('#');
    let slug = slug.split('?').next().unwrap_or(slug);
    ALL_OPS_PAGES
        .iter()
        .copied()
        .find(|p| p.slug() == slug)
        .unwrap_or(OpsPage::Dashboard)
}

/// Parse a deep-link sub-id from the hash fragment.
/// Format: `#<slug>?id=<case-id>`
pub(crate) fn ops_sub_id_from_hash(hash: &str) -> Option<String> {
    let hash = hash.trim_start_matches('#');
    let query = hash.split_once('?')?.1;
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("id=") {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub(crate) fn ops_set_hash(page: OpsPage) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(page.slug());
    }
}

/// Set the URL hash with a deep-link sub-id, e.g. `#investigate?id=CASE-001`.
pub(crate) fn ops_set_hash_with_id(page: OpsPage, sub_id: &str) {
    if let Some(window) = web_sys::window() {
        let hash = if sub_id.is_empty() {
            page.slug().to_string()
        } else {
            format!("{}?id={}", page.slug(), sub_id)
        };
        let _ = window.location().set_hash(&hash);
    }
}
