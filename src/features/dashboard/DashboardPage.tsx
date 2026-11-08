import { lazy, Suspense } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router-dom";
import { AlertTriangle, ArrowDown, ArrowUp, Plus, TrendingDown, TrendingUp } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { api, errorMessage, type Dashboard } from "@/lib/api";
import { formatDate, monthLong, todayISO } from "@/lib/dates";
import { formatMoney } from "@/lib/money";
import { cn } from "@/lib/utils";

// Recharts adalah bagian terbesar; kartu angka tampil dulu, grafik menyusul begitu modulnya siap.
const RevenueExpenseChart = lazy(() =>
  import("./charts").then((m) => ({ default: m.RevenueExpenseChart })),
);
const ExpenseDonut = lazy(() => import("./charts").then((m) => ({ default: m.ExpenseDonut })));
const CashChart = lazy(() => import("./charts").then((m) => ({ default: m.CashChart })));

function ChartFallback() {
  return <div className="h-72 animate-pulse rounded-xl border bg-muted/30" aria-hidden />;
}

/** Dashboard: kartu ringkasan bulan berjalan, tiga chart, dan jurnal terakhir. */
export function DashboardPage() {
  const navigate = useNavigate();
  const today = todayISO();
  const { data, error, isLoading } = useQuery({
    queryKey: ["dashboard", today],
    queryFn: () => api.getDashboard(today),
  });
  const info = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Dashboard</h1>
          <p className="text-sm text-muted-foreground">
            {data
              ? `Ringkasan ${monthLong(data.month)} · per ${formatDate(data.today)}`
              : "Memuat..."}
          </p>
        </div>
        <Button onClick={() => navigate("/jurnal/baru")}>
          <Plus className="size-4" /> Jurnal baru
        </Button>
      </div>

      {error && <p className="text-sm text-destructive">{errorMessage(error)}</p>}
      {isLoading && <p className="text-sm text-muted-foreground">Memuat...</p>}

      {data && data.draftCount > 0 && (
        <Link
          to="/jurnal"
          className="flex items-center gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-800 dark:text-amber-300"
          data-draft-banner
        >
          <AlertTriangle className="size-4" />
          {data.draftCount} jurnal masih berstatus draft dan belum dihitung di laporan.
        </Link>
      )}

      {data && !data.hasData && <EmptyState />}

      {data && data.hasData && (
        <>
          <Kpis d={data} />
          <div className="grid gap-4 lg:grid-cols-5">
            <div className="min-w-0 lg:col-span-3">
              <Suspense fallback={<ChartFallback />}>
                <RevenueExpenseChart data={data.monthly} />
              </Suspense>
            </div>
            <div className="min-w-0 lg:col-span-2">
              <Suspense fallback={<ChartFallback />}>
                <ExpenseDonut data={data.expenseBreakdown} total={data.expense} />
              </Suspense>
            </div>
          </div>
          <div className="grid gap-4 lg:grid-cols-5">
            <div className="min-w-0 lg:col-span-3">
              <Suspense fallback={<ChartFallback />}>
                <CashChart data={data.monthly} />
              </Suspense>
            </div>
            <div className="min-w-0 lg:col-span-2">
              <RecentJournals d={data} />
            </div>
          </div>
        </>
      )}

      {info.data && (
        <p className="text-xs text-muted-foreground" data-system-info>
          Versi {info.data.version} · skema database {info.data.schemaVersion} · {info.data.dbPath}
        </p>
      )}
    </div>
  );
}

function EmptyState() {
  return (
    <div className="rounded-xl border border-dashed px-6 py-14 text-center" data-empty-state>
      <h2 className="text-lg font-medium">Belum ada transaksi yang diposting</h2>
      <p className="mx-auto mt-1 max-w-md text-sm text-muted-foreground">
        Isi saldo awal bila Anda pindah dari pembukuan lama, atau langsung catat jurnal pertama.
        Ringkasan dan grafik muncul setelah ada jurnal yang diposting.
      </p>
      <div className="mt-5 flex justify-center gap-2">
        <Button variant="outline" render={<Link to="/saldo-awal" />}>
          Isi saldo awal
        </Button>
        <Button render={<Link to="/jurnal/baru" />}>Buat jurnal pertama</Button>
      </div>
    </div>
  );
}

/** Selisih terhadap bulan lalu, ditulis dengan panah + persen (tidak bergantung pada warna). */
function Delta({ cur, prev }: { cur: number; prev: number }) {
  if (prev === 0) {
    return <span className="text-xs text-muted-foreground">Bulan lalu: {formatMoney(prev)}</span>;
  }
  const pct = ((cur - prev) / Math.abs(prev)) * 100;
  if (Math.abs(pct) < 0.05)
    return <span className="text-xs text-muted-foreground">Sama dengan bulan lalu</span>;
  const Icon = pct > 0 ? ArrowUp : ArrowDown;
  return (
    <span className="flex items-center gap-1 text-xs text-muted-foreground" data-delta>
      <Icon className="size-3" aria-hidden />
      {Math.abs(pct).toLocaleString("id-ID", { maximumFractionDigits: 1 })}%{" "}
      {pct > 0 ? "naik" : "turun"} dari bulan lalu
    </span>
  );
}

function Kpi({
  id,
  label,
  value,
  children,
  tone,
}: {
  id: string;
  label: React.ReactNode;
  value: string;
  children?: React.ReactNode;
  tone?: "good" | "bad";
}) {
  return (
    <div className="rounded-xl border bg-card p-4 text-card-foreground" data-kpi={id}>
      <div className="text-xs text-muted-foreground">{label}</div>
      <div
        className={cn(
          "mt-1 text-xl font-semibold tabular-nums",
          tone === "good" && "text-emerald-700 dark:text-emerald-400",
          tone === "bad" && "text-destructive",
        )}
        data-kpi-value
      >
        {value}
      </div>
      <div className="mt-1.5 min-h-4">{children}</div>
    </div>
  );
}

function Kpis({ d }: { d: Dashboard }) {
  const profit = d.net >= 0;
  const NetIcon = profit ? TrendingUp : TrendingDown;
  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      <Kpi id="cash" label="Kas & Bank" value={formatMoney(d.cashBalance)}>
        {d.cashAccounts.length === 0 ? (
          <span className="text-xs text-muted-foreground">
            Tandai akun kas/bank di Chart of Accounts
          </span>
        ) : (
          <ul className="space-y-0.5 text-xs text-muted-foreground">
            {d.cashAccounts.slice(0, 3).map((a) => (
              <li key={a.id} className="flex justify-between gap-2">
                <span className="truncate">{a.name}</span>
                <span className="tabular-nums">{formatMoney(a.balance)}</span>
              </li>
            ))}
            {d.cashAccounts.length > 3 && <li>+{d.cashAccounts.length - 3} akun lainnya</li>}
          </ul>
        )}
      </Kpi>
      <Kpi id="revenue" label="Pendapatan bulan ini" value={formatMoney(d.revenue)}>
        <Delta cur={d.revenue} prev={d.prevRevenue} />
      </Kpi>
      <Kpi id="expense" label="Beban bulan ini" value={formatMoney(d.expense)}>
        <Delta cur={d.expense} prev={d.prevExpense} />
      </Kpi>
      <Kpi
        id="net"
        label={
          <span className="flex items-center gap-1">
            <NetIcon className="size-3.5" aria-hidden />
            {profit ? "Laba bersih bulan ini" : "Rugi bersih bulan ini"}
          </span>
        }
        value={formatMoney(Math.abs(d.net))}
        tone={profit ? "good" : "bad"}
      >
        <Delta cur={d.net} prev={d.prevNet} />
      </Kpi>
    </div>
  );
}

function RecentJournals({ d }: { d: Dashboard }) {
  return (
    <section className="h-full rounded-xl border bg-card p-5 text-card-foreground" data-recent>
      <header className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-medium">Jurnal terakhir</h2>
        <Link to="/jurnal" className="text-xs text-muted-foreground hover:text-foreground">
          Lihat semua
        </Link>
      </header>
      <ul className="divide-y">
        {d.recent.map((j) => (
          <li key={j.id}>
            <Link
              to={j.type === "OPENING" ? "/saldo-awal" : `/jurnal/${j.id}`}
              className="flex items-center gap-3 py-2 text-sm hover:text-foreground"
            >
              <div className="min-w-0 flex-1">
                <div className="truncate">{j.description}</div>
                <div className="text-xs text-muted-foreground">
                  {formatDate(j.date)} · <span className="font-mono">{j.number}</span>
                </div>
              </div>
              <div className="text-right">
                <div className="text-sm tabular-nums">{formatMoney(j.total)}</div>
                <Badge variant={j.status === "POSTED" ? "secondary" : "outline"}>
                  {j.status === "POSTED" ? "Diposting" : "Draft"}
                </Badge>
              </div>
            </Link>
          </li>
        ))}
      </ul>
    </section>
  );
}
