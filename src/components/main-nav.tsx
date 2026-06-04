import {
  Book02Icon,
  CheckmarkBadge01Icon,
  Moon02Icon,
  Settings01Icon,
  Sun03Icon,
  TranslationIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Link } from "@tanstack/react-router";
import { useTheme } from "@/components/theme-provider";
import { Button } from "@/components/ui/button";

const links = [
  { to: "/glossary", label: "Glossary", icon: Book02Icon },
  { to: "/translate", label: "Translate", icon: TranslationIcon },
  { to: "/verify", label: "Verify", icon: CheckmarkBadge01Icon },
  { to: "/settings", label: "Settings", icon: Settings01Icon },
] as const;

export function MainNav() {
  const { resolvedTheme, setTheme } = useTheme();
  const isDark = resolvedTheme === "dark";

  return (
    <header className="flex items-center gap-1 border-b px-3 py-2">
      <span className="px-2 text-sm font-semibold tracking-tight">
        polygluttony
      </span>
      <nav className="flex items-center gap-1">
        {links.map(({ to, label, icon }) => (
          <Button key={to} asChild variant="ghost" size="sm">
            <Link
              to={to}
              activeProps={{ "data-active": "true" }}
              className="data-[active=true]:bg-accent data-[active=true]:text-accent-foreground"
            >
              <HugeiconsIcon icon={icon} strokeWidth={2} data-icon="inline-start" />
              {label}
            </Link>
          </Button>
        ))}
      </nav>
      <Button
        variant="ghost"
        size="icon"
        className="ml-auto"
        aria-label="Toggle theme"
        onClick={() => setTheme(isDark ? "light" : "dark")}
      >
        <HugeiconsIcon icon={isDark ? Sun03Icon : Moon02Icon} strokeWidth={2} />
      </Button>
    </header>
  );
}
