import { useQuery } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router-dom";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { AccountPicker } from "@/components/AccountPicker";
import { ExportButtons } from "@/components/ExportButtons";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api, errorMessage } from "@/lib/api";
import { formatDate, todayISO } from "@/lib/dates";
import { formatMoney, formatMoneyOrBlank } from "@/lib/money";
import { cn } from "@/lib/utils";

const PAGE_SIZE = 100;

/**
 * Buku Besar: mutasi satu akun dalam rentang tanggal beserta saldo berjalan.
 * Pilihan disimpan di URL (?akun=&dari=&sampai=&hal=) supaya bisa ditautkan dari halaman lain.
 */
export function LedgerPage() {
  const [params, setParams] = useSearchParams();
  const accountId = params.get("akun") ? Number(params.get("akun")) : null;
  const from = params.get("dari") ?? `${new Date().getFullYear()}-01-01`;
  const to = params.get("sampai") ?? todayISO();
  const page = Math.max(1, Number(params.get("hal") ?? 1) || 1);

  const accounts = useQuery({ queryKey: ["accounts"], queryFn: api.listAccounts });
  const ledger = useQuery({
    queryKey: ["ledger", accountId, from, to, page],
    queryFn: () => api.getLedger({ accountId: accountId!, from, to, page, pageSize: PAGE_SIZE }),
    enabled: accountId !== null && from <= to,
    placeholderData: (prev) => prev,
  });

  function update(patch: Record<string, string | null>) {
    const next = new URLSearchParams(params);
    for (const [k, v] of Object.entries(patch)) {
      if (v === null || v === "") next.delete(k);
      else next.set(k, v);
    }
    setParams(next, { replace: true });
  }

  const data = ledger.data;
  const totalPages = data ? Math.max(1, Math.ceil(data.total / data.pageSize)) : 1;
  const showAccountCol = data?.account.isHeader ?? false;
  const cols = showAccountCol ? 7 : 6;

  return (
    <div className="space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Buku Besar</h1>
        <p className="text-sm text-muted-foreground">
          Mutasi per akun dengan saldo berjalan. Hanya jurnal yang sudah diposting yang dihitung.
        </p>
      </div>

      <div className="flex flex-wrap items-end gap-3">
        <label className="flex w-80 flex-col gap-1 text-xs text-muted-foreground">
          Akun
          {accounts.data ? (
            <AccountPicker
              cell="ledger-account"
              options={accounts.data.filter((a) => a.isActive)}
              all={accounts.data}
              value={accountId}
              onChange={(id) => update({ akun: id === null ? null : String(id), hal: null })}
            />
          ) : (
            <Input disabled placeholder="Memuat..." />
          )}
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Dari
          <Input
            type="date"
            className="w-40"
            value={from}
            onChange={(e) => update({ dari: e.target.value, hal: null })}
          />
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Sampai
          <Input
            type="date"
            className="w-40"
            value={to}
            onChange={(e) => update({ sampai: e.target.value, hal: null })}
          />
        </label>
        {accountId !== null && (
          <div className="ml-auto">
            <ExportButtons
              disabled={from > to}
              name={`buku-besar-${data?.account.code ?? accountId}-${from}_${to}`}
              request={{ kind: "ledger", accountId, from, to }}
            />
          </div>
        )}
      </div>

      {from > to && (
        <p className="text-sm text-destructive">Tanggal awal tidak boleh setelah tanggal akhir.</p>
      )}
      {ledger.error && <p className="text-sm text-destructive">{errorMessage(ledger.error)}</p>}

      {accountId === null && (
        <p className="rounded-xl border border-dashed px-4 py-12 text-center text-sm text-muted-foreground">
          Pilih akun untuk melihat buku besarnya.
        </p>
      )}

      {data && (
        <>
          <div className="grid grid-cols-4 gap-3">
            <Stat label="Saldo awal" value={data.openingBalance} id="opening" />
            <Stat label="Total debit" value={data.totalDebit} id="debit" />
            <Stat label="Total kredit" value={data.totalCredit} id="credit" />
            <Stat label="Saldo akhir" value={data.closingBalance} id="closing" strong />
          </div>

          <div className="rounded-xl border">
            <table className="w-full text-sm">
              <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
                <tr>
                  <th className="w-32 rounded-tl-xl px-4 py-2.5 font-medium">Tanggal</th>
                  <th className="w-44 px-4 py-2.5 font-medium">Nomor</th>
                  {showAccountCol && <th className="w-24 px-4 py-2.5 font-medium">Akun</th>}
                  <th className="px-4 py-2.5 font-medium">Keterangan</th>
                  <th className="w-36 px-4 py-2.5 text-right font-medium">Debit</th>
                  <th className="w-36 px-4 py-2.5 text-right font-medium">Kredit</th>
                  <th className="w-40 rounded-tr-xl px-4 py-2.5 text-right font-medium">Saldo</th>
                </tr>
              </thead>
              <tbody>
                {data.page === 1 && (
                  <tr className="border-t bg-muted/20 text-muted-foreground" data-row="opening">
                    <td className="px-4 py-2.5 whitespace-nowrap">
                      {data.from ? formatDate(data.from) : ""}
                    </td>
                    <td colSpan={cols - 4} className="px-4 py-2.5 italic">
                      Saldo awal
                    </td>
                    <td />
                    <td />
                    <td className="px-4 py-2.5 text-right tabular-nums">
                      {formatMoney(data.openingBalance)}
                    </td>
                  </tr>
                )}
                {data.rows.length === 0 && (
                  <tr className="border-t">
                    <td colSpan={cols} className="px-4 py-10 text-center text-muted-foreground">
                      Tidak ada mutasi pada rentang tanggal ini.
                    </td>
                  </tr>
                )}
                {data.rows.map((r, i) => (
                  <tr key={`${r.entryId}-${i}`} className="border-t" data-row="mutation">
                    <td className="px-4 py-2.5 whitespace-nowrap">{formatDate(r.date)}</td>
                    <td className="px-4 py-2.5">
                      <Link
                        to={`/jurnal/${r.entryId}`}
                        className="font-mono text-xs underline-offset-2 hover:underline"
                      >
                        {r.number}
                      </Link>
                    </td>
                    {showAccountCol && (
                      <td className="px-4 py-2.5 font-mono text-xs" title={r.accountName}>
                        {r.accountCode}
                      </td>
                    )}
                    <td className="px-4 py-2.5">
                      {r.description}
                      {r.memo && <div className="text-xs text-muted-foreground">{r.memo}</div>}
                    </td>
                    <td className="px-4 py-2.5 text-right tabular-nums">
                      {formatMoneyOrBlank(r.debit)}
                    </td>
                    <td className="px-4 py-2.5 text-right tabular-nums">
                      {formatMoneyOrBlank(r.credit)}
                    </td>
                    <td
                      className={cn(
                        "px-4 py-2.5 text-right tabular-nums",
                        r.balance < 0 && "text-destructive",
                      )}
                    >
                      {formatMoney(r.balance)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {data.total > 0 && (
            <div className="flex items-center justify-between text-sm text-muted-foreground">
              <span>
                {data.total} baris · Halaman {data.page} dari {totalPages}
              </span>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={page <= 1}
                  onClick={() => update({ hal: String(page - 1) })}
                >
                  <ChevronLeft className="size-4" /> Sebelumnya
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={page >= totalPages}
                  onClick={() => update({ hal: String(page + 1) })}
                >
                  Berikutnya <ChevronRight className="size-4" />
                </Button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function Stat({
  label,
  value,
  id,
  strong,
}: {
  label: string;
  value: number;
  id: string;
  strong?: boolean;
}) {
  return (
    <div className="rounded-xl border p-4">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div
        data-stat={id}
        className={cn(
          "mt-1 text-lg tabular-nums",
          strong && "font-semibold",
          value < 0 && "text-destructive",
        )}
      >
        {formatMoney(value)}
      </div>
    </div>
  );
}
