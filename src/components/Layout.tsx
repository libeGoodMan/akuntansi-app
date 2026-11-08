import { Suspense, useEffect, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Keyboard, Moon, Search, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useTheme } from "@/lib/theme";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { NAV_ITEMS } from "@/components/nav";
import { CommandPalette } from "@/features/palette/CommandPalette";
import { ShortcutsDialog } from "@/features/palette/ShortcutsDialog";
import { UpdateBanner } from "@/features/update/UpdateBanner";

/** Pintasan ketik di kolom isian tidak boleh memicu pintasan global satu tombol. */
function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName));
}

export function Layout() {
  const { theme, toggle } = useTheme();
  const navigate = useNavigate();
  const location = useLocation();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setHelpOpen(false);
        setPaletteOpen((o) => !o);
      } else if (e.altKey && !e.ctrlKey && !e.metaKey) {
        const n = Number(e.key);
        if (n >= 1 && n <= NAV_ITEMS.length) {
          e.preventDefault();
          navigate(NAV_ITEMS[n - 1].to);
        } else if (e.key.toLowerCase() === "n") {
          e.preventDefault();
          navigate("/jurnal/baru");
        }
      } else if (e.key === "?" && !isTyping(e.target) && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        setHelpOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate]);

  return (
    <div className="flex h-screen bg-background text-foreground">
      <aside className="flex w-60 shrink-0 flex-col border-r bg-sidebar text-sidebar-foreground">
        <div className="px-5 py-4 text-lg font-semibold tracking-tight">Akuntansi</div>
        <div className="px-3 pb-3">
          <button
            type="button"
            data-open-palette
            onClick={() => setPaletteOpen(true)}
            className="flex w-full items-center gap-2 rounded-lg border bg-background/50 px-3 py-1.5 text-sm text-muted-foreground hover:bg-sidebar-accent/60"
          >
            <Search className="size-4" aria-hidden />
            <span className="flex-1 text-left">Cari...</span>
            <kbd className="rounded border bg-muted px-1.5 font-mono text-[10px]">Ctrl K</kbd>
          </button>
        </div>
        <nav className="flex-1 space-y-0.5 px-3">
          {NAV_ITEMS.map(({ to, label, icon: Icon }, i) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              title={`${label} (Alt+${i + 1})`}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                  isActive
                    ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground"
                    : "text-sidebar-foreground/70 hover:bg-sidebar-accent/60",
                )
              }
            >
              <Icon className="size-4" />
              {label}
            </NavLink>
          ))}
        </nav>
        <div className="space-y-1 border-t p-3">
          <Button
            variant="ghost"
            size="sm"
            className="w-full justify-start gap-3"
            onClick={() => setHelpOpen(true)}
          >
            <Keyboard className="size-4" />
            Pintasan keyboard
          </Button>
          <Button variant="ghost" size="sm" className="w-full justify-start gap-3" onClick={toggle}>
            {theme === "dark" ? <Sun className="size-4" /> : <Moon className="size-4" />}
            {theme === "dark" ? "Tema terang" : "Tema gelap"}
          </Button>
        </div>
      </aside>
      <div className="flex min-w-0 flex-1 flex-col">
        <UpdateBanner />
        <main className="flex-1 overflow-auto p-8">
          <ErrorBoundary resetKey={location.pathname}>
            <Suspense fallback={<p className="text-sm text-muted-foreground">Memuat...</p>}>
              <Outlet />
            </Suspense>
          </ErrorBoundary>
        </main>
      </div>

      <CommandPalette
        open={paletteOpen}
        onOpenChange={setPaletteOpen}
        onToggleTheme={toggle}
        onShowShortcuts={() => setHelpOpen(true)}
      />
      <ShortcutsDialog open={helpOpen} onOpenChange={setHelpOpen} />
    </div>
  );
}
