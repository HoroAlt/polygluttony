import { createFileRoute } from "@tanstack/react-router";
import { TranslatePage } from "@/features/translate/translate-page";

export const Route = createFileRoute("/translate")({
  component: TranslatePage,
});
