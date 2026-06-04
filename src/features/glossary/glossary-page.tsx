import { PageHeader } from "@/components/page-header";

export function GlossaryPage() {
  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <PageHeader
        title="Glossary"
        description="Build and refine the six-category terminology glossary that keeps names and terms consistent across every file."
      />
      {/* Glossary table, build controls, and diff review land here. */}
    </div>
  );
}
