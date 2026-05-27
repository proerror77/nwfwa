import { useState } from "react";
import { RuntimeScoring } from "./pages/RuntimeScoring";
import { PlannedModulePage } from "./pages/PlannedModulePage";
import { RulesStudio } from "./pages/RulesStudio";
import { ModelOpsPage } from "./pages/ModelOpsPage";
import { KnowledgeBasePage } from "./pages/KnowledgeBasePage";
import { AgentInvestigatorPage } from "./pages/AgentInvestigatorPage";

const modules = [
  "Dashboard",
  "Runtime Scoring",
  "Rules",
  "Models",
  "Factor Factory",
  "Knowledge Base",
  "Agent Investigator",
  "QA Review",
  "Governance",
];

export function App() {
  const [active, setActive] = useState("Runtime Scoring");
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
        {active === "Runtime Scoring" ? <RuntimeScoring /> : null}
        {active === "Rules" ? <RulesStudio /> : null}
        {active === "Models" ? <ModelOpsPage /> : null}
        {active === "Knowledge Base" ? <KnowledgeBasePage /> : null}
        {active === "Agent Investigator" ? <AgentInvestigatorPage /> : null}
        {!["Runtime Scoring", "Rules", "Models", "Knowledge Base", "Agent Investigator"].includes(
          active,
        ) ? (
          <PlannedModulePage title={active} />
        ) : null}
      </main>
    </div>
  );
}
