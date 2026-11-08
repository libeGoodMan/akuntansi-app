import { useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import type { Account } from "@/lib/api";
import { cn } from "@/lib/utils";

interface Props {
  /** Akun yang boleh dipilih (non-header, aktif). */
  options: Account[];
  /** Semua akun, untuk menampilkan label akun terpilih yang mungkin sudah nonaktif. */
  all: Account[];
  value: number | null;
  onChange: (id: number | null) => void;
  /** Penanda untuk fokus programatik / pengujian. */
  cell?: string;
  disabled?: boolean;
}

const label = (a: Account) => `${a.code} ${a.name}`;

/** Kolom akun dengan autocomplete: ketik kode atau nama, Enter/Tab untuk memilih. */
export function AccountPicker({ options, all, value, onChange, cell, disabled }: Props) {
  const selected = all.find((a) => a.id === value) ?? null;
  const [query, setQuery] = useState<string | null>(null); // null = tidak sedang mengetik
  const [open, setOpen] = useState(false);
  const [hi, setHi] = useState(0);

  const text = query ?? (selected ? label(selected) : "");

  const matches = useMemo(() => {
    const tokens = (query ?? "").toLowerCase().split(/\s+/).filter(Boolean);
    return options
      .filter((a) => {
        const hay = label(a).toLowerCase();
        return tokens.every((t) => hay.includes(t));
      })
      .slice(0, 50);
  }, [options, query]);

  function choose(a: Account) {
    onChange(a.id);
    setQuery(null);
    setOpen(false);
  }

  return (
    <div className="relative">
      <Input
        data-cell={cell}
        disabled={disabled}
        value={text}
        placeholder="Pilih akun"
        autoComplete="off"
        onFocus={(e) => {
          e.currentTarget.select();
          setOpen(true);
        }}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
          setHi(0);
        }}
        onBlur={() => {
          if (query !== null && query.trim() === "") onChange(null);
          setQuery(null);
          setOpen(false);
        }}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setOpen(true);
            setHi((h) => Math.min(h + 1, matches.length - 1));
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setHi((h) => Math.max(h - 1, 0));
          } else if (e.key === "Enter" && open && matches[hi] && query !== null) {
            e.preventDefault();
            e.stopPropagation();
            choose(matches[hi]);
          } else if (e.key === "Tab" && query !== null && matches[hi]) {
            choose(matches[hi]); // fokus tetap pindah seperti biasa
          } else if (e.key === "Escape" && open) {
            e.stopPropagation();
            setQuery(null);
            setOpen(false);
          }
        }}
      />
      {open && !disabled && (
        <ul
          role="listbox"
          className="absolute top-full right-0 left-0 z-30 mt-1 max-h-60 overflow-auto rounded-lg border bg-popover p-1 text-sm text-popover-foreground shadow-md"
        >
          {matches.length === 0 && (
            <li className="px-2 py-1.5 text-muted-foreground">Tidak ada akun yang cocok</li>
          )}
          {matches.map((a, i) => (
            <li
              key={a.id}
              role="option"
              aria-selected={a.id === value}
              // mousedown (bukan click) supaya input tidak kehilangan fokus sebelum pilihan terbaca
              onMouseDown={(e) => {
                e.preventDefault();
                choose(a);
              }}
              onMouseEnter={() => setHi(i)}
              className={cn(
                "flex cursor-pointer gap-2 rounded-md px-2 py-1.5",
                i === hi && "bg-muted",
                a.id === value && "font-medium",
              )}
            >
              <span className="w-16 shrink-0 font-mono text-xs leading-5 text-muted-foreground">
                {a.code}
              </span>
              <span>{a.name}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
