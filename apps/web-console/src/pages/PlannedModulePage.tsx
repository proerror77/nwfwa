export function PlannedModulePage({ title }: { title: string }) {
  return (
    <section className="panel">
      <h2>{title}</h2>
      <p>This module is planned for a later phase. No production API is available yet.</p>
    </section>
  );
}
