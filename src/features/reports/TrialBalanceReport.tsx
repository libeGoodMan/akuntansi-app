import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { ExportButtons } from "@/components/ExportButtons";
import { Input } from "@/components/ui/input";
import {
  ACCOUNT_TYPE_LABEL,
  api,
  errorMessage,
  type AccountType,
  type TrialBalanceRow,
} from "@/lib/api";
import { todayISO } from "@/lib/dates";
import { formatMoney, formatMoneyOrBlank } from "@/lib/money";
import { cn } from "@/lib/utils";

const TYPE_ORDER: AccountType[] = ["ASSET", "LIABILITY", "EQUITY", "REVENUE", "EXPENSE"];

/** Neraca Saldo per tanggal: saldo semua akun di kolom debit/kredit; total harus sama. */
export function TrialBalanceReport() {
  const navigate = useNavigate();
  const [asOf, setAsOf] = useState(todayISO());
  const [includeZero, setIncludeZero] = useState(false);

  const { data, error, isLoading } = useQuery({
    queryKey: ["trial-balance", asOf, includeZero],
    queryFn: () => api.getTrialBalance({ asOf: asOf || undefined, includeZero }),
    placeholderData: (prev) => prev,
  });

  const groups = TYPE_ORDER.map((t) => ({
    type: t,
    rows: (data?.rows ?? []).filter((r) => r.type === t),
  })).filter((g) => g.rows.length > 0);

  function openLedger(r: TrialBalanceRow) {
    const q = new URLSearchParams({ akun: String(r.accountId) });
    if (asOf) q.set("sampai", asOf);
    navigate(`/buku-besar?${q}`);
  }

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-end gap-4">
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Per tanggal
          <Input
            type="date"
            className="w-40"
            value={asOf}
            onChange={(e) => setAsOf(e.target.value)}
          />
        </label>
        <label className="flex items-center gap-2 pb-2 text-sm">
          <input
            type="checkbox"
            checked={includeZero}
            onChange={(e) => setIncludeZero(e.target.checked)}
          />
          Tampilkan akun bersaldo nol
        </label>
        <div className="ml-auto">
          <ExportButtons
            name={`neraca-saldo-${asOf || "semua"}`}
            request={{ kind: "trial-balance", asOf: asOf || undefined, includeZero }}
          />
        </div>
      </div>

      {error && <p className="text-sm text-destructive">{errorMessage(error)}</p>}

      <div className="rounded-xl border">
        <table className="w-full text-sm">
          <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
            <tr>
              <th className="w-28 rounded-tl-xl px-4 py-2.5 font-medium">Kode</th>
              <th className="px-4 py-2.5 font-medium">Nama akun</th>
              <th className="w-44 px-4 py-2.5 text-right font-medium">Debit</th>
              <th className="w-44 rounded-tr-xl px-4 py-2.5 text-right font-medium">Kredit</th>
            </tr>
          </thead>
          <tbody>
            {isLoading && (
              <tr>
                <td colSpan={4} className="px-4 py-10 text-center text-muted-foreground">
                  Memuat...
                </td>
              </tr>
            )}
            {data && data.rows.length === 0 && (
              <tr>
                <td colSpan={4} className="px-4 py-10 text-center text-muted-foreground">
                  Belum ada saldo pada tanggal ini.
                </td>
              </tr>
            )}
            {groups.map((g) => (
              <GroupRows key={g.type} type={g.type} rows={g.rows} onOpen={openLedger} />
            ))}
          </tbody>
          {data && data.rows.length > 0 && (
            <tfoot>
              <tr className="border-t-2 bg-muted/30 font-semibold">
                <td colSpan={2} className="px-4 py-3">
                  Total
                </td>
                <td className="px-4 py-3 text-right tabular-nums" data-total="debit">
                  {formatMoney(data.totalDebit)}
                </td>
                <td className="px-4 py-3 text-right tabular-nums" data-total="credit">
                  {formatMoney(data.totalCredit)}
                </td>
              </tr>
            </tfoot>
          )}
        </table>
      </div>

      {data && data.rows.length > 0 && (
        <p
          data-balance={data.balanced ? "ok" : "off"}
          className={cn(
            "text-sm",
            data.balanced ? "text-emerald-600 dark:text-emerald-400" : "text-destructive",
          )}
        >
          {data.balanced
            ? "Seimbang: total debit sama dengan total kredit."
            : `Tidak seimbang, selisih ${formatMoney(Math.abs(data.difference))}. Periksa jurnal.`}
        </p>
      )}
    </div>
  );
}

function GroupRows({
  type,
  rows,
  onOpen,
}: {
  type: AccountType;
  rows: TrialBalanceRow[];
  onOpen: (r: TrialBalanceRow) => void;
}) {
  return (
    <>
      <tr className="border-t bg-muted/20">
        <td colSpan={4} className="px-4 py-1.5 text-xs font-medium text-muted-foreground">
          {ACCOUNT_TYPE_LABEL[type]}
        </td>
      </tr>
      {rows.map((r) => (
        <tr
          key={r.accountId}
          data-account={r.code}
          tabIndex={0}
          title="Buka buku besar akun ini"
          className="cursor-pointer border-t hover:bg-muted/40 focus-visible:bg-muted/40 focus-visible:outline-none"
          onClick={() => onOpen(r)}
          onKeyDown={(e) => e.key === "Enter" && onOpen(r)}
        >
          <td className="px-4 py-2 font-mono text-xs">{r.code}</td>
          <td className={cn("px-4 py-2", !r.isActive && "text-muted-foreground")}>
            {r.name}
            {!r.isActive && <span className="ml-2 text-xs">(nonaktif)</span>}
          </td>
          <td className="px-4 py-2 text-right tabular-nums">
            {formatMoneyOrBlank(r.balanceDebit)}
          </td>
          <td className="px-4 py-2 text-right tabular-nums">
            {formatMoneyOrBlank(r.balanceCredit)}
          </td>
        </tr>
      ))}
    </>
  );
}
