import { useSearchParams } from "react-router-dom";
import { cn } from "@/lib/utils";
import { BalanceSheetReport } from "./BalanceSheetReport";
import { IncomeStatementReport } from "./IncomeStatementReport";
import { TrialBalanceReport } from "./TrialBalanceReport";

const TABS = [
  { id: "neraca-saldo", label: "Neraca Saldo" },
  { id: "laba-rugi", label: "Laba Rugi" },
  { id: "neraca", label: "Neraca" },
] as const;

type TabId = (typeof TABS)[number]["id"];

/** Wadah laporan keuangan; tab aktif disimpan di URL (?tab=). */
export function ReportsPage() {
  const [params, setParams] = useSearchParams();
  const requested = params.get("tab");
  const tab: TabId = TABS.some((t) => t.id === requested) ? (requested as TabId) : "neraca-saldo";

  return (
    <div className="space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Laporan</h1>
        <p className="text-sm text-muted-foreground">
          Seluruh angka dihitung langsung dari jurnal yang sudah diposting. Tiap laporan bisa
          dicetak, disimpan sebagai PDF, atau diekspor ke Excel.
        </p>
      </div>

      <div className="flex gap-1 border-b" role="tablist">
        {TABS.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={tab === t.id}
            onClick={() => setParams({ tab: t.id }, { replace: true })}
            className={cn(
              "-mb-px border-b-2 px-3 py-2 text-sm transition-colors",
              tab === t.id
                ? "border-primary font-medium"
                : "border-transparent text-muted-foreground hover:text-foreground",
            )}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === "neraca-saldo" && <TrialBalanceReport />}
      {tab === "laba-rugi" && <IncomeStatementReport />}
      {tab === "neraca" && <BalanceSheetReport />}
    </div>
  );
}
