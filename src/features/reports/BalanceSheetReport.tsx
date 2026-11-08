import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { ExportButtons } from "@/components/ExportButtons";
import { Input } from "@/components/ui/input";
import { api, errorMessage, type StatementLine } from "@/lib/api";
import { todayISO } from "@/lib/dates";
import { formatMoney } from "@/lib/money";
import { cn } from "@/lib/utils";
import { StatementTable } from "./StatementTable";

/** Neraca per tanggal: Aset = Kewajiban + Ekuitas (termasuk laba berjalan). */
export function BalanceSheetReport() {
  const navigate = useNavigate();
  const [asOf, setAsOf] = useState(todayISO());

  const { data, error, isLoading } = useQuery({
    queryKey: ["balance-sheet", asOf],
    queryFn: () => api.getBalanceSheet(asOf || undefined),
    placeholderData: (prev) => prev,
  });

  function openAccount(l: StatementLine) {
    const q = new URLSearchParams({ akun: String(l.accountId) });
    if (asOf) q.set("sampai", asOf);
    navigate(`/buku-besar?${q}`);
  }

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Per tanggal
          <Input
            type="date"
            className="w-40"
            value={asOf}
            onChange={(e) => setAsOf(e.target.value)}
          />
        </label>
        <ExportButtons
          name={`neraca-${asOf || "semua"}`}
          request={{ kind: "balance-sheet", asOf: asOf || undefined }}
        />
      </div>

      {error && <p className="text-sm text-destructive">{errorMessage(error)}</p>}
      {isLoading && <p className="text-sm text-muted-foreground">Memuat...</p>}

      {data && (
        <>
          <StatementTable lines={data.lines} valueLabel="Jumlah" onOpenAccount={openAccount} />
          <p
            data-balance={data.balanced ? "ok" : "off"}
            className={cn(
              "text-sm",
              data.balanced ? "text-emerald-600 dark:text-emerald-400" : "text-destructive",
            )}
          >
            {data.balanced
              ? "Seimbang: Aset sama dengan Kewajiban ditambah Ekuitas."
              : `Tidak seimbang, selisih ${formatMoney(Math.abs(data.difference))}. Periksa jurnal.`}
          </p>
        </>
      )}
    </div>
  );
}
