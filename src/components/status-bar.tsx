import { Folder01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import { Separator } from "@/components/ui/separator";
import { ipc } from "@/lib/ipc";
import { useAppStore } from "@/stores/app-store";

/** Bottom status strip: working folder, language pair, connection, core version. */
export function StatusBar() {
  const workdir = useAppStore((s) => s.workdir);
  const sourceLang = useAppStore((s) => s.sourceLang);
  const targetLang = useAppStore((s) => s.targetLang);
  const connection = useAppStore((s) => s.activeConnection);

  // Exercises the IPC bridge end-to-end (invoke + ts-rs generated type).
  const { data: appInfo } = useQuery({
    queryKey: ["app-info"],
    queryFn: ipc.appInfo,
  });

  return (
    <footer className="flex h-8 shrink-0 items-center gap-3 border-t px-3 text-xs text-muted-foreground">
      <span className="flex min-w-0 items-center gap-1.5">
        <HugeiconsIcon
          icon={Folder01Icon}
          strokeWidth={2}
          className="size-3.5 shrink-0"
        />
        <span className="truncate">{workdir ?? "No folder selected"}</span>
      </span>
      <Separator orientation="vertical" className="h-4" />
      <span className="shrink-0">
        {sourceLang} → {targetLang}
      </span>
      <Separator orientation="vertical" className="h-4" />
      <span className="shrink-0">{connection ?? "No connection"}</span>
      <span className="ml-auto shrink-0">core {appInfo?.version ?? "…"}</span>
    </footer>
  );
}
