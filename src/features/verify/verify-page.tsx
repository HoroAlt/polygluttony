import { PageHeader } from "@/components/page-header";

export function VerifyPage() {
  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <PageHeader
        title="Verify"
        description="Run LLM-based sampling verification on translated files and flag low-scoring ones for retranslation."
      />
      {/* Scored results table and issue breakdown land here. */}
    </div>
  );
}
