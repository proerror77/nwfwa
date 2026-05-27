import { RuntimeScoring } from "./pages/RuntimeScoring";
import { PlannedModulePage } from "./pages/PlannedModulePage";

const modules = [
  "Dashboard",
  "Runtime Scoring",
  "Rules",
  "Models",
  "Factor Factory",
  "Knowledge Base",
  "QA Review",
  "Governance",
];

export function App() {
  const active = "Runtime Scoring";
  return (
    <div className="app">
      <aside>
        <h1>FWA Studio</h1>
        {modules.map((module) => (
          <button className={module === active ? "active" : ""} key={module}>
            {module}
          </button>
        ))}
      </aside>
      <main>
        {active === "Runtime Scoring" ? <RuntimeScoring /> : <PlannedModulePage title={active} />}
      </main>
    </div>
  );
}
