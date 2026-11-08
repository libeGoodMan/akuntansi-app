import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import {
  BookOpen,
  DatabaseBackup,
  Keyboard,
  Moon,
  NotebookPen,
  Plus,
  RefreshCw,
  Search,
  type LucideIcon,
} from "lucide-react";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { api, errorMessage } from "@/lib/api";
import { formatDate } from "@/lib/dates";
import { cn } from "@/lib/utils";
import { NAV_ITEMS } from "@/components/nav";

interface Item {
  id: string;
  group: "Halaman" | "Aksi" | "Akun" | "Jurnal";
  label: string;
  hint?: string;
  icon: LucideIcon;
  run: () => void;
}

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onToggleTheme: () => void;
  onShowShortcuts: () => void;
}

const norm = (s: string) => s.toLowerCase();

/** Palet perintah (Ctrl+K): loncat ke halaman, jalankan aksi, atau cari akun dan jurnal. */
export function CommandPalette({ open, onOpenChange, onToggleTheme, onShowShortcuts }: Props) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="top-[20%] translate-y-0 gap-0 p-0 sm:max-w-xl" data-palette>
        <DialogTitle className="sr-only">Palet perintah</DialogTitle>
        <DialogDescription className="sr-only">
          Cari halaman, aksi, akun, atau jurnal
        </DialogDescription>
        {/* Hanya terpasang saat terbuka, sehingga isian dan pilihan selalu mulai bersih. */}
        {open && (
          <PaletteBody
            onClose={() => onOpenChange(false)}
            onToggleTheme={onToggleTheme}
            onShowShortcuts={onShowShortcuts}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

interface BodyProps {
  onClose: () => void;
  onToggleTheme: () => void;
  onShowShortcuts: () => void;
}

function PaletteBody({ onClose, onToggleTheme, onShowShortcuts }: BodyProps) {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  // Tunda pencarian jurnal 200 ms supaya tidak query di setiap ketikan.
  useEffect(() => {
    const t = setTimeout(() => setDebounced(query.trim()), 200);
    return () => clearTimeout(t);
  }, [query]);

  const accounts = useQuery({ queryKey: ["accounts"], queryFn: api.listAccounts });
  const journals = useQuery({
    queryKey: ["palette-journals", debounced],
    queryFn: () => api.listJournals({ search: debounced, page: 1, pageSize: 6 }),
    enabled: debounced.length >= 2,
  });

  const items = useMemo<Item[]>(() => {
    const go = (to: string) => () => navigate(to);
    const q = norm(query.trim());
    const match = (s: string) => q === "" || norm(s).includes(q);

    const pages: Item[] = NAV_ITEMS.map((n, i) => ({ n, i }))
      .filter(({ n }) => match(n.label))
      .map(({ n, i }) => ({
        id: `page:${n.to}`,
        group: "Halaman" as const,
        label: n.label,
        hint: `Alt+${i + 1}`,
        icon: n.icon,
        run: go(n.to),
      }));

    const actions: Item[] = (
      [
        {
          id: "new-journal",
          label: "Jurnal baru",
          hint: "Alt+N",
          icon: Plus,
          run: go("/jurnal/baru"),
        },
        {
          id: "backup",
          label: "Backup data sekarang",
          icon: DatabaseBackup,
          run: () =>
            api
              .createBackup()
              .then(() => toast.success("Backup dibuat"))
              .catch((e) => toast.error(errorMessage(e))),
        },
        { id: "theme", label: "Ganti tema terang/gelap", icon: Moon, run: onToggleTheme },
        { id: "update", label: "Periksa pembaruan", icon: RefreshCw, run: go("/pengaturan") },
        { id: "keys", label: "Pintasan keyboard", hint: "?", icon: Keyboard, run: onShowShortcuts },
      ] satisfies Omit<Item, "group">[]
    )
      .filter((a) => match(a.label))
      .map((a) => ({ ...a, id: `action:${a.id}`, group: "Aksi" as const }));

    const acc: Item[] =
      q === ""
        ? []
        : (accounts.data ?? [])
            .filter((a) => norm(`${a.code} ${a.name}`).includes(q))
            .slice(0, 6)
            .map((a) => ({
              id: `account:${a.id}`,
              group: "Akun" as const,
              label: `${a.code} ${a.name}`,
              hint: "Buku besar",
              icon: BookOpen,
              run: go(`/buku-besar?akun=${a.id}`),
            }));

    const jr: Item[] =
      debounced.length < 2
        ? []
        : (journals.data?.items ?? []).map((j) => ({
            id: `journal:${j.id}`,
            group: "Jurnal" as const,
            label: `${j.number}  ${j.description}`,
            hint: formatDate(j.date),
            icon: NotebookPen,
            run: go(j.type === "OPENING" ? "/saldo-awal" : `/jurnal/${j.id}`),
          }));

    return [...pages, ...actions, ...acc, ...jr];
  }, [query, debounced, accounts.data, journals.data, navigate, onToggleTheme, onShowShortcuts]);

  useEffect(() => {
    listRef.current
      ?.querySelector<HTMLElement>(`[data-index="${active}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [active]);

  function run(item: Item | undefined) {
    if (!item) return;
    onClose();
    item.run();
  }

  let lastGroup = "";
  return (
    <>
      <div className="flex items-center gap-2 border-b px-4">
        <Search className="size-4 text-muted-foreground" aria-hidden />
        <input
          autoFocus
          data-palette-input
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setActive((a) => Math.min(a + 1, items.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setActive((a) => Math.max(a - 1, 0));
            } else if (e.key === "Enter") {
              e.preventDefault();
              run(items[active]);
            }
          }}
          placeholder="Ketik untuk mencari halaman, aksi, akun, atau nomor jurnal..."
          className="h-12 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
        />
      </div>
      <div ref={listRef} className="max-h-80 overflow-auto p-2" role="listbox">
        {items.length === 0 && (
          <p className="px-3 py-8 text-center text-sm text-muted-foreground" data-palette-empty>
            Tidak ada hasil untuk “{query}”.
          </p>
        )}
        {items.map((it, i) => {
          const header = it.group !== lastGroup;
          lastGroup = it.group;
          return (
            <div key={it.id}>
              {header && (
                <div className="px-3 pt-2 pb-1 text-xs font-medium text-muted-foreground">
                  {it.group}
                </div>
              )}
              <div
                role="option"
                aria-selected={i === active}
                data-index={i}
                data-item={it.id}
                onMouseMove={() => setActive(i)}
                onClick={() => run(it)}
                className={cn(
                  "flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 text-sm",
                  i === active && "bg-muted",
                )}
              >
                <it.icon className="size-4 shrink-0 text-muted-foreground" aria-hidden />
                <span className="flex-1 truncate">{it.label}</span>
                {it.hint && <span className="text-xs text-muted-foreground">{it.hint}</span>}
              </div>
            </div>
          );
        })}
      </div>
      <div className="flex justify-between border-t px-4 py-2 text-xs text-muted-foreground">
        <span>↑↓ pilih · Enter jalankan · Esc tutup</span>
        <span>Ctrl+K</span>
      </div>
    </>
  );
}
