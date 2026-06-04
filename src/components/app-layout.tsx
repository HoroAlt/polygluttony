import type { ReactNode } from "react";
import { MainNav } from "@/components/main-nav";
import { StatusBar } from "@/components/status-bar";

/** Top-level window chrome: nav header, routed content, and the status strip. */
export function AppLayout({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <MainNav />
      <main className="min-h-0 flex-1 overflow-auto">{children}</main>
      <StatusBar />
    </div>
  );
}
