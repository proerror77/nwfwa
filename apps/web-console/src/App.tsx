import { useState } from "react";
import { RuntimeScoring } from "./pages/RuntimeScoring";
import { PlannedModulePage } from "./pages/PlannedModulePage";
import { RulesStudio } from "./pages/RulesStudio";
import { ModelOpsPage } from "./pages/ModelOpsPage";
import { KnowledgeBasePage } from "./pages/KnowledgeBasePage";
import { AgentInvestigatorPage } from "./pages/AgentInvestigatorPage";
import { DataSourcesPage } from "./pages/DataSourcesPage";
import { DashboardPage } from "./pages/DashboardPage";
import { QAReviewPage } from "./pages/QAReviewPage";
import { FactorFactoryPage } from "./pages/FactorFactoryPage";
import { LeadsCasesPage } from "./pages/LeadsCasesPage";
import { GovernancePage } from "./pages/GovernancePage";

const modules = [
  "Dashboard",
  "Runtime Scoring",
  "Rules",
  "Models",
  "Data Sources",
  "Factor Factory",
  "Leads & Cases",
  "Knowledge Base",
  "Agent Investigator",
  "QA Review",
  "Governance",
];

export function App() {
  const [active, setActive] = useState("Dashboard");
  return (
    <div className="app">
      <aside>
        <h1>FWA Studio</h1>
        {modules.map((module) => (
          <button
            className={module === active ? "active" : ""}
            key={module}
            onClick={() => setActive(module)}
          >
            {module}
          </button>
        ))}
      </aside>
      <main>
        {active === "Dashboard" ? <DashboardPage /> : null}
        {active === "Runtime Scoring" ? <RuntimeScoring /> : null}
        {active === "Rules" ? <RulesStudio /> : null}
        {active === "Models" ? <ModelOpsPage /> : null}
        {active === "Data Sources" ? <DataSourcesPage /> : null}
        {active === "Factor Factory" ? <FactorFactoryPage /> : null}
        {active === "Leads & Cases" ? <LeadsCasesPage /> : null}
        {active === "Knowledge Base" ? <KnowledgeBasePage /> : null}
        {active === "Agent Investigator" ? <AgentInvestigatorPage /> : null}
        {active === "QA Review" ? <QAReviewPage /> : null}
        {active === "Governance" ? <GovernancePage /> : null}
        {![
          "Runtime Scoring",
          "Dashboard",
          "Rules",
          "Models",
          "Data Sources",
          "Factor Factory",
          "Leads & Cases",
          "Knowledge Base",
          "Agent Investigator",
          "QA Review",
          "Governance",
        ].includes(active) ? (
          <PlannedModulePage title={active} />
        ) : null}
      </main>
    </div>
  );
}
