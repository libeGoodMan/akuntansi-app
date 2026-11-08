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

/** Geser tahun pada YYYY-MM-DD (untuk periode pembanding tahun lalu). */
function shiftYear(iso: string, delta: number): string {
  const [y, ...rest] = iso.split("-");
  return [String(Number(y) + delta), ...rest].join("-");
}

/** Laba Rugi untuk rentang tanggal, dengan kolom pembanding (periode lain) opsional. */
export function IncomeStatementReport() {
  const navigate = useNavigate();
  const today = todayISO();
  const [from, setFrom] = useState(`${today.slice(0, 4)}-01-01`);
  const [to, setTo] = useState(today);
  const [compare, setCompare] = useState(false);
  const [cFrom, setCFrom] = useState(shiftYear(`${today.slice(0, 4)}-01-01`, -1));
  const [cTo, setCTo] = useState(shiftYear(today, -1));

  const rangeOk = from <= to && (!compare || cFrom <= cTo);
  const query = {
    from,
    to,
    compareFrom: compare ? cFrom : undefined,
    compareTo: compare ? cTo : undefined,
  };
  const { data, error, isLoading } = useQuery({
    queryKey: ["income-statement", query],
    queryFn: () => api.getIncomeStatement(query),
    enabled: rangeOk,
    placeholderData: (prev) => prev,
  });

  function openAccount(l: StatementLine) {
    const q = new URLSearchParams({ akun: String(l.accountId), dari: from, sampai: to });
    navigate(`/buku-besar?${q}`);
  }

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div className="flex flex-wrap items-end gap-4">
          <label className="flex flex-col gap-1 text-xs text-muted-foreground">
            Dari
            <Input
              type="date"
              className="w-40"
              value={from}
              onChange={(e) => setFrom(e.target.value)}
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-muted-foreground">
            Sampai
            <Input
              type="date"
              className="w-40"
              value={to}
              onChange={(e) => setTo(e.target.value)}
            />
          </label>
          <label className="flex items-center gap-2 pb-2 text-sm">
            <input
              type="checkbox"
              checked={compare}
              onChange={(e) => setCompare(e.target.checked)}
            />
            Bandingkan dengan periode lain
          </label>
          {compare && (
            <>
              <label className="flex flex-col gap-1 text-xs text-muted-foreground">
                Pembanding: dari
                <Input
                  type="date"
                  className="w-40"
                  value={cFrom}
                  onChange={(e) => setCFrom(e.target.value)}
                />
              </label>
              <label className="flex flex-col gap-1 text-xs text-muted-foreground">
                sampai
                <Input
                  type="date"
                  className="w-40"
                  value={cTo}
                  onChange={(e) => setCTo(e.target.value)}
                />
              </label>
            </>
          )}
        </div>
        <ExportButtons
          disabled={!rangeOk}
          name={`laba-rugi-${from}_${to}`}
          request={{
            kind: "income-statement",
            from,
            to,
            compareFrom: compare ? cFrom : undefined,
            compareTo: compare ? cTo : undefined,
          }}
        />
      </div>

      {!rangeOk && (
        <p className="text-sm text-destructive">Tanggal awal tidak boleh setelah tanggal akhir.</p>
      )}
      {error && <p className="text-sm text-destructive">{errorMessage(error)}</p>}
      {isLoading && <p className="text-sm text-muted-foreground">Memuat...</p>}

      {data && (
        <>
          <StatementTable
            lines={data.lines}
            hasCompare={data.hasCompare}
            valueLabel="Periode ini"
            compareLabel="Pembanding"
            onOpenAccount={openAccount}
          />
          <p
            data-net-income
            className={cn(
              "text-sm",
              data.netIncome < 0 ? "text-destructive" : "text-muted-foreground",
            )}
          >
            {data.netIncome >= 0 ? "Laba bersih" : "Rugi bersih"} periode ini:{" "}
            <span className="font-medium tabular-nums">
              {formatMoney(Math.abs(data.netIncome))}
            </span>
            . Jurnal penutup akhir tahun tidak ikut dihitung di laporan ini.
          </p>
        </>
      )}
    </div>
  );
}
