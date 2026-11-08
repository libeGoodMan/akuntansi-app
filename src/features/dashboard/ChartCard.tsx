import { useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface LegendItem {
  label: string;
  /** Nama variabel CSS warna, mis. "--series-1". */
  color: string;
}

export interface TableView {
  columns: string[];
  rows: string[][];
}

interface Props {
  title: string;
  subtitle?: string;
  /** Legenda selalu ada untuk >= 2 seri; satu seri cukup diberi judul. */
  legend?: LegendItem[];
  /** Tampilan tabel: jalur alternatif bagi pembaca yang tidak bisa membedakan warna. */
  table: TableView;
  className?: string;
  children: ReactNode;
}

/** Kartu chart dengan judul, legenda, dan tombol pengalih grafik/tabel. */
export function ChartCard({ title, subtitle, legend, table, className, children }: Props) {
  const [showTable, setShowTable] = useState(false);

  return (
    <section
      className={cn("rounded-xl border bg-card p-5 text-card-foreground", className)}
      data-chart={title}
    >
      <header className="mb-4 flex items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-medium">{title}</h2>
          {subtitle && <p className="text-xs text-muted-foreground">{subtitle}</p>}
        </div>
        <button
          type="button"
          aria-pressed={showTable}
          onClick={() => setShowTable((v) => !v)}
          className="rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          {showTable ? "Grafik" : "Tabel"}
        </button>
      </header>

      {legend && legend.length > 1 && !showTable && (
        <ul className="mb-3 flex flex-wrap gap-x-4 gap-y-1 text-xs" aria-label="Legenda">
          {legend.map((l) => (
            <li key={l.label} className="flex items-center gap-1.5 text-muted-foreground">
              <span
                className="size-2.5 rounded-sm"
                style={{ background: `var(${l.color})` }}
                aria-hidden
              />
              {l.label}
            </li>
          ))}
        </ul>
      )}

      {showTable ? (
        <div className="max-h-72 overflow-auto rounded-lg border">
          <table className="w-full text-xs" data-chart-table>
            <thead className="sticky top-0 bg-muted text-left text-muted-foreground">
              <tr>
                {table.columns.map((c, i) => (
                  <th key={c} className={cn("px-3 py-2 font-medium", i > 0 && "text-right")}>
                    {c}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {table.rows.map((r, i) => (
                <tr key={i} className="border-t">
                  {r.map((cell, j) => (
                    <td key={j} className={cn("px-3 py-1.5 tabular-nums", j > 0 && "text-right")}>
                      {cell}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        children
      )}
    </section>
  );
}
