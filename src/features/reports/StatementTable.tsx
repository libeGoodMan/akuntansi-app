import type { StatementLine } from "@/lib/api";
import { formatMoney } from "@/lib/money";
import { cn } from "@/lib/utils";

interface Props {
  lines: StatementLine[];
  hasCompare?: boolean;
  valueLabel: string;
  compareLabel?: string;
  /** Dipanggil saat baris akun diklik (membuka buku besarnya). */
  onOpenAccount?: (line: StatementLine) => void;
}

const money = (v: number | null) => (v === null ? "" : formatMoney(v));

/** Tabel laporan keuangan dari daftar baris datar (susunan sama dengan PDF dan Excel). */
export function StatementTable({
  lines,
  hasCompare,
  valueLabel,
  compareLabel,
  onOpenAccount,
}: Props) {
  const cols = hasCompare ? 5 : 3;
  return (
    <div className="rounded-xl border">
      <table className="w-full text-sm">
        <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
          <tr>
            <th className="w-28 rounded-tl-xl px-4 py-2.5 font-medium">Kode</th>
            <th className="px-4 py-2.5 font-medium">Keterangan</th>
            <th
              className={cn(
                "w-44 px-4 py-2.5 text-right font-medium",
                !hasCompare && "rounded-tr-xl",
              )}
            >
              {valueLabel}
            </th>
            {hasCompare && (
              <>
                <th className="w-44 px-4 py-2.5 text-right font-medium">{compareLabel}</th>
                <th className="w-44 rounded-tr-xl px-4 py-2.5 text-right font-medium">Selisih</th>
              </>
            )}
          </tr>
        </thead>
        <tbody>
          {lines.length === 0 && (
            <tr>
              <td colSpan={cols} className="px-4 py-10 text-center text-muted-foreground">
                Belum ada data pada periode ini.
              </td>
            </tr>
          )}
          {lines.map((l, i) => {
            const pad = { paddingLeft: `${(l.level - 1) * 1.25}rem` };
            if (l.kind === "heading") {
              return (
                <tr key={i} className={cn("border-t", l.level === 1 && "bg-muted/40")}>
                  <td colSpan={cols} className="px-4 py-2 font-semibold">
                    <span style={pad}>{l.label}</span>
                  </td>
                </tr>
              );
            }
            const strong = l.kind !== "account";
            const clickable = l.kind === "account" && l.accountId !== null && onOpenAccount;
            const change = l.amount !== null && l.compare !== null ? l.amount - l.compare : null;
            const negative = (v: number | null) => v !== null && v < 0;
            return (
              <tr
                key={i}
                data-line={l.label}
                data-kind={l.kind}
                className={cn(
                  "border-t",
                  strong && "font-semibold",
                  l.kind === "net" && "border-t-2 bg-muted/50",
                  l.kind === "total" && "bg-muted/20",
                  clickable && "cursor-pointer hover:bg-muted/40",
                )}
                onClick={clickable ? () => onOpenAccount(l) : undefined}
                title={clickable ? "Buka buku besar akun ini" : undefined}
              >
                <td className="px-4 py-2 font-mono text-xs text-muted-foreground">{l.code}</td>
                <td className="px-4 py-2">
                  <span style={pad}>{l.label}</span>
                </td>
                <td
                  className={cn(
                    "px-4 py-2 text-right tabular-nums",
                    negative(l.amount) && l.kind === "net" && "text-destructive",
                  )}
                  data-col="amount"
                >
                  {money(l.amount)}
                </td>
                {hasCompare && (
                  <>
                    <td className="px-4 py-2 text-right tabular-nums" data-col="compare">
                      {money(l.compare)}
                    </td>
                    <td className="px-4 py-2 text-right tabular-nums" data-col="change">
                      {money(change)}
                    </td>
                  </>
                )}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
