import { PageHeader } from "@/components/page-header";

export function SettingsPage() {
  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <PageHeader
        title="Settings"
        description="Manage LLM connections, default languages, and pipeline preferences. Saved to the local Tauri store."
      />
      {/* Connection editor (driver, base URL, key, model, concurrency) lands here. */}
    </div>
  );
}
