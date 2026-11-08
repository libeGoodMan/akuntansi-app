import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { ExpenseSlice, MonthPoint } from "@/lib/api";
import { monthLong, monthShort } from "@/lib/dates";
import { formatCompact, formatMoney } from "@/lib/money";
import { ChartCard } from "./ChartCard";

/** "2026-11" -> "Nov": 12 bulan tidak pernah memuat nama bulan yang sama dua kali. */
const monthName = (ym: string) => monthShort(ym).split(" ")[0];

const AXIS_TICK = { fill: "var(--chart-axis)", fontSize: 11 };
const SERIES_VARS = [
  "--series-1",
  "--series-2",
  "--series-3",
  "--series-4",
  "--series-5",
  "--series-6",
];

interface TipEntry {
  name?: string | number;
  value?: number | string;
  color?: string;
  dataKey?: string | number;
  payload?: { month?: string };
}

/** Tooltip: bulan di judul, satu baris per seri dengan nominal lengkap. */
function MoneyTooltip({ active, payload }: { active?: boolean; payload?: readonly TipEntry[] }) {
  if (!active || !payload?.length) return null;
  const month = payload[0]?.payload?.month;
  return (
    <div className="rounded-lg border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md">
      {month && <div className="mb-1 font-medium">{monthLong(month)}</div>}
      {payload.map((p) => (
        <div key={String(p.dataKey)} className="flex items-center justify-between gap-4">
          <span className="flex items-center gap-1.5 text-muted-foreground">
            <span className="size-2 rounded-sm" style={{ background: p.color }} aria-hidden />
            {p.name}
          </span>
          <span className="tabular-nums">{formatMoney(Number(p.value))}</span>
        </div>
      ))}
    </div>
  );
}

/** Pendapatan vs beban, 12 bulan (batang berpasangan). */
export function RevenueExpenseChart({ data }: { data: MonthPoint[] }) {
  return (
    <ChartCard
      title="Pendapatan vs beban"
      subtitle="12 bulan terakhir, tidak termasuk jurnal penutup"
      legend={[
        { label: "Pendapatan", color: "--series-1" },
        { label: "Beban", color: "--series-2" },
      ]}
      table={{
        columns: ["Bulan", "Pendapatan", "Beban", "Laba/Rugi"],
        rows: data.map((m) => [
          monthLong(m.month),
          formatMoney(m.revenue),
          formatMoney(m.expense),
          formatMoney(m.net),
        ]),
      }}
    >
      <div className="h-64" role="img" aria-label="Grafik batang pendapatan dan beban per bulan">
        <ResponsiveContainer width="100%" height="100%" minWidth={0}>
          <BarChart
            data={data}
            margin={{ top: 8, right: 8, left: 0, bottom: 0 }}
            barGap={2}
            barCategoryGap="24%"
          >
            <CartesianGrid vertical={false} stroke="var(--chart-grid)" />
            <XAxis
              dataKey="month"
              tickFormatter={monthName}
              tickLine={false}
              axisLine={false}
              tick={AXIS_TICK}
              interval={0}
            />
            <YAxis
              tickFormatter={formatCompact}
              tickLine={false}
              axisLine={false}
              tick={AXIS_TICK}
              width={58}
            />
            <Tooltip content={<MoneyTooltip />} cursor={{ fill: "var(--muted)", opacity: 0.5 }} />
            <Bar
              dataKey="revenue"
              name="Pendapatan"
              fill="var(--series-1)"
              radius={[4, 4, 0, 0]}
              maxBarSize={16}
            />
            <Bar
              dataKey="expense"
              name="Beban"
              fill="var(--series-2)"
              radius={[4, 4, 0, 0]}
              maxBarSize={16}
            />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </ChartCard>
  );
}

/** Saldo kas & bank pada akhir tiap bulan (satu seri: tanpa legenda, judul yang menamai). */
export function CashChart({ data }: { data: MonthPoint[] }) {
  return (
    <ChartCard
      title="Saldo kas & bank"
      subtitle="Akhir bulan, 12 bulan terakhir"
      table={{
        columns: ["Bulan", "Saldo"],
        rows: data.map((m) => [monthLong(m.month), formatMoney(m.cash)]),
      }}
    >
      <div className="h-64" role="img" aria-label="Grafik garis saldo kas dan bank per bulan">
        <ResponsiveContainer width="100%" height="100%" minWidth={0}>
          <LineChart data={data} margin={{ top: 8, right: 12, left: 0, bottom: 0 }}>
            <CartesianGrid vertical={false} stroke="var(--chart-grid)" />
            <XAxis
              dataKey="month"
              tickFormatter={monthName}
              tickLine={false}
              axisLine={false}
              tick={AXIS_TICK}
              interval={0}
            />
            <YAxis
              tickFormatter={formatCompact}
              tickLine={false}
              axisLine={false}
              tick={AXIS_TICK}
              width={58}
            />
            <Tooltip
              content={<MoneyTooltip />}
              cursor={{ stroke: "var(--chart-axis)", strokeDasharray: "3 3" }}
            />
            <Line
              type="linear"
              dataKey="cash"
              name="Saldo kas & bank"
              stroke="var(--series-1)"
              strokeWidth={2}
              dot={{ r: 3.5, fill: "var(--series-1)", stroke: "var(--card)", strokeWidth: 2 }}
              activeDot={{ r: 5, fill: "var(--series-1)", stroke: "var(--card)", strokeWidth: 2 }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </ChartCard>
  );
}

const sliceVar = (i: number, s: ExpenseSlice) =>
  s.code === null ? "--series-other" : SERIES_VARS[i % SERIES_VARS.length];

/** Komposisi beban bulan berjalan: donat + daftar berlabel nominal dan persen (identitas tidak hanya lewat warna). */
export function ExpenseDonut({ data, total }: { data: ExpenseSlice[]; total: number }) {
  const pct = (v: number) =>
    total > 0
      ? `${((v / total) * 100).toLocaleString("id-ID", { maximumFractionDigits: 1 })}%`
      : "0%";

  return (
    <ChartCard
      title="Komposisi beban"
      subtitle="Bulan berjalan"
      table={{
        columns: ["Akun", "Jumlah", "Porsi"],
        rows: data.map((s) => [s.label, formatMoney(s.amount), pct(s.amount)]),
      }}
    >
      {data.length === 0 ? (
        <p className="flex h-64 items-center justify-center text-sm text-muted-foreground">
          Belum ada beban bulan ini.
        </p>
      ) : (
        <div className="flex flex-col items-center gap-4">
          <div
            className="relative size-44 shrink-0"
            role="img"
            aria-label="Grafik donat komposisi beban bulan ini"
          >
            <ResponsiveContainer width="100%" height="100%" minWidth={0}>
              <PieChart>
                <Pie
                  data={data}
                  dataKey="amount"
                  nameKey="label"
                  innerRadius={54}
                  outerRadius={84}
                  paddingAngle={2}
                  stroke="var(--card)"
                  strokeWidth={2}
                  isAnimationActive={false}
                >
                  {data.map((s, i) => (
                    <Cell key={s.label} fill={`var(${sliceVar(i, s)})`} />
                  ))}
                </Pie>
                <Tooltip
                  content={({ active, payload }) =>
                    active && payload?.length ? (
                      <div className="rounded-lg border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md">
                        <div className="font-medium">{String(payload[0].name)}</div>
                        <div className="tabular-nums">
                          {formatMoney(Number(payload[0].value))} · {pct(Number(payload[0].value))}
                        </div>
                      </div>
                    ) : null
                  }
                />
              </PieChart>
            </ResponsiveContainer>
            <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center">
              <span className="text-[10px] text-muted-foreground">Total beban</span>
              <span className="text-sm font-semibold tabular-nums">{formatCompact(total)}</span>
            </div>
          </div>

          <ul className="w-full space-y-1.5 text-sm" data-expense-legend>
            {data.map((s, i) => (
              <li key={s.label} className="flex items-center gap-2">
                <span
                  className="size-2.5 shrink-0 rounded-sm"
                  style={{ background: `var(${sliceVar(i, s)})` }}
                  aria-hidden
                />
                <span className="min-w-0 flex-1 truncate" title={s.label}>
                  {s.label}
                </span>
                <span className="tabular-nums text-muted-foreground">
                  {formatCompact(s.amount)}
                </span>
                <span className="w-12 text-right text-xs tabular-nums text-muted-foreground">
                  {pct(s.amount)}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </ChartCard>
  );
}
