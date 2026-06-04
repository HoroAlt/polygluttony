import { PageHeader } from "@/components/page-header";

export function TranslatePage() {
  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <PageHeader
        title="Translate"
        description="Run the three-pass pipeline over a subtitle folder, with per-file progress, retries, and drift detection."
      />
      {/* Tone selection, per-file progress table, and log panel land here. */}
    </div>
  );
}
