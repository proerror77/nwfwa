/// System Learning page — three governance tabs that help operators understand
/// how the FWA system is performing and decide what to tune.
///
/// Tab 1: Rule Candidates  — new fraud patterns discovered, backtest, promotion
/// Tab 2: Model Governance — model versions, drift, activation decisions
/// Tab 3: QA / Feedback    — quality assurance feedback loop
use crate::i18n::tr;
use crate::pages::{ModelsPage, QaReviewPage, RulesPage};
use crate::state::Language;
use yew::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GovTab {
    Rules,
    Models,
    Qa,
}

impl GovTab {
    fn label(self, language: Language) -> &'static str {
        match (self, language) {
            (GovTab::Rules, Language::En) => "Rule Candidates & Patterns",
            (GovTab::Rules, Language::Zh) => "规则候选 & 新模式",
            (GovTab::Models, Language::En) => "Model Governance",
            (GovTab::Models, Language::Zh) => "模型治理",
            (GovTab::Qa, Language::En) => "QA Feedback Loop",
            (GovTab::Qa, Language::Zh) => "QA 反馈闭环",
        }
    }

    fn description(self, language: Language) -> &'static str {
        match (self, language) {
            (GovTab::Rules, Language::En) => "Review candidate rules discovered from claims data. Approve new fraud patterns before they go live.",
            (GovTab::Rules, Language::Zh) => "审核从理赔数据中发现的候选规则。新诈骗模式上线前需人工批准。",
            (GovTab::Models, Language::En) => "Track model versions, evaluation metrics, drift alerts, and activation decisions.",
            (GovTab::Models, Language::Zh) => "查看模型版本、评估指标、漂移警告和激活决策。",
            (GovTab::Qa, Language::En) => "Close reviewer feedback into rule and model improvements. Prevents errors from compounding.",
            (GovTab::Qa, Language::Zh) => "闭环复核反馈，回流规则和模型改进，防止误差累积。",
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct SystemLearningPageProps {
    pub language: Language,
}

#[function_component(SystemLearningPage)]
pub fn system_learning_page(props: &SystemLearningPageProps) -> Html {
    let active_tab = use_state(|| GovTab::Rules);
    let language = props.language;
    let tab = *active_tab;

    html! {
        <div class="ops-page system-learning-page">
            <div class="ops-page-header">
                <div>
                    <h2>{tr(language, "System Governance", "系统治理")}</h2>
                    <p class="muted">{tr(
                        language,
                        "New fraud patterns, model updates, and QA feedback — the loop that keeps the FWA system improving.",
                        "新诈骗模式发现、模型更新与 QA 反馈——让 FWA 系统持续改进的闭环。"
                    )}</p>
                </div>
            </div>

            // ── Tab bar ─────────────────────────────────────────────────────
            <div class="tab-bar">
                { [GovTab::Rules, GovTab::Models, GovTab::Qa].iter().map(|&t| {
                    let active_tab = active_tab.clone();
                    let is_active = tab == t;
                    html! {
                        <button
                            class={classes!("tab-btn", is_active.then_some("active"))}
                            onclick={Callback::from(move |_: MouseEvent| active_tab.set(t))}
                        >
                            {t.label(language)}
                        </button>
                    }
                }).collect::<Html>() }
            </div>

            // ── Tab description bar ─────────────────────────────────────────
            <div class="tab-description">
                <p class="muted">{tab.description(language)}</p>
            </div>

            // ── Tab body — reuse existing full-featured pages ───────────────
            <div class="tab-body tab-body-full">
                { match tab {
                    GovTab::Rules  => html! { <RulesPage /> },
                    GovTab::Models => html! { <ModelsPage /> },
                    GovTab::Qa     => html! { <QaReviewPage /> },
                } }
            </div>
        </div>
    }
}
