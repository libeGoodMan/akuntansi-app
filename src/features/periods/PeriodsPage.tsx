import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { toast } from "sonner";
import { CheckCircle2, Lock, LockOpen } from "lucide-react";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { NativeSelect } from "@/components/ui/native-select";
import { api, errorMessage, type MonthStatus } from "@/lib/api";
import { formatDate, formatDateTime, monthLong, todayISO } from "@/lib/dates";
import { formatMoney } from "@/lib/money";

type Confirm =
  | { kind: "close-month"; m: MonthStatus }
  | { kind: "reopen-month"; m: MonthStatus }
  | { kind: "close-year" }
  | { kind: "reopen-year" };

const monthKey = (m: MonthStatus) => `${m.year}-${String(m.month).padStart(2, "0")}`;

/** Periode: tutup/buka bulan dan tutup buku tahunan, per tahun fiskal. */
export function PeriodsPage() {
  const queryClient = useQueryClient();
  const [year, setYear] = useState<number | null>(null);
  const [confirm, setConfirm] = useState<Confirm | null>(null);
  const [error, setError] = useState<string | null>(null);

  const currentFy = useQuery({
    queryKey: ["fiscal-year-now"],
    queryFn: () => api.fiscalYearFor(todayISO()),
  });
  const fy = year ?? currentFy.data ?? null;

  const overview = useQuery({
    queryKey: ["period-overview", fy],
    queryFn: () => api.periodOverview(fy!),
    enabled: fy !== null,
  });

  const act = useMutation({
    mutationFn: async (c: Confirm) => {
      switch (c.kind) {
        case "close-month":
          return api.closePeriod(c.m.year, c.m.month);
        case "reopen-month":
          return api.reopenPeriod(c.m.year, c.m.month);
        case "close-year":
          return api.closeYear(fy!);
        case "reopen-year":
          return api.reopenYear(fy!);
      }
    },
    onSuccess: async (_, c) => {
      setError(null);
      setConfirm(null);
      // Status periode memengaruhi jurnal, laporan, dan dashboard.
      await queryClient.invalidateQueries();
      toast.success(successText(c, fy));
    },
    onError: (e) => {
      setConfirm(null);
      setError(errorMessage(e));
    },
  });

  const ov = overview.data;
  const now = currentFy.data;
  const years = now ? Array.from({ length: 8 }, (_, i) => now - 5 + i) : [];

  return (
    <div className="mx-auto max-w-4xl space-y-6">
      <div className="flex items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Periode</h1>
          <p className="text-sm text-muted-foreground">
            Periode yang ditutup tidak bisa diubah jurnalnya. Setelah 12 bulan ditutup, tutup buku
            tahunan memindahkan laba/rugi ke Laba Ditahan.
          </p>
        </div>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Tahun fiskal
          <NativeSelect
            className="w-36"
            value={fy ?? ""}
            onChange={(e) => setYear(Number(e.target.value))}
          >
            {years.map((y) => (
              <option key={y} value={y}>
                {y}
              </option>
            ))}
          </NativeSelect>
        </label>
      </div>

      {error && (
        <p
          role="alert"
          className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
        >
          {error}
        </p>
      )}
      {overview.error && <p className="text-sm text-destructive">{errorMessage(overview.error)}</p>}

      {ov && (
        <>
          <p className="text-sm text-muted-foreground">
            {formatDate(ov.start)} s.d. {formatDate(ov.end)}
          </p>

          <div className="rounded-xl border">
            <table className="w-full text-sm">
              <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
                <tr>
                  <th className="rounded-tl-xl px-4 py-2.5 font-medium">Periode</th>
                  <th className="w-32 px-4 py-2.5 font-medium">Status</th>
                  <th className="w-24 px-4 py-2.5 text-right font-medium">Jurnal</th>
                  <th className="w-24 px-4 py-2.5 text-right font-medium">Draft</th>
                  <th className="w-40 rounded-tr-xl px-4 py-2.5 text-right font-medium">Aksi</th>
                </tr>
              </thead>
              <tbody>
                {ov.months.map((m) => (
                  <tr
                    key={monthKey(m)}
                    className="border-t"
                    data-period={monthKey(m)}
                    data-closed={m.closed}
                  >
                    <td className="px-4 py-2.5">{monthLong(monthKey(m))}</td>
                    <td className="px-4 py-2.5">
                      {m.closed ? (
                        <Badge variant="secondary">
                          <Lock className="size-3" /> Ditutup
                        </Badge>
                      ) : (
                        <Badge variant="outline">
                          <LockOpen className="size-3" /> Terbuka
                        </Badge>
                      )}
                    </td>
                    <td className="px-4 py-2.5 text-right tabular-nums">{m.posted}</td>
                    <td className="px-4 py-2.5 text-right tabular-nums">
                      {m.drafts > 0 ? (
                        <span className="text-amber-700 dark:text-amber-400">{m.drafts}</span>
                      ) : (
                        0
                      )}
                    </td>
                    <td className="px-4 py-1.5 text-right">
                      {m.closed ? (
                        <Button
                          size="sm"
                          variant="ghost"
                          data-action="reopen"
                          disabled={ov.closing !== null || act.isPending}
                          title={ov.closing ? "Batalkan tutup buku tahunan dulu" : undefined}
                          onClick={() => setConfirm({ kind: "reopen-month", m })}
                        >
                          Buka kembali
                        </Button>
                      ) : (
                        <Button
                          size="sm"
                          variant="outline"
                          data-action="close"
                          disabled={m.drafts > 0 || act.isPending}
                          title={m.drafts > 0 ? "Posting atau hapus jurnal draft dulu" : undefined}
                          onClick={() => setConfirm({ kind: "close-month", m })}
                        >
                          Tutup
                        </Button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <section className="space-y-3 rounded-xl border p-5" data-year-card>
            <div className="flex items-start justify-between gap-4">
              <div>
                <h2 className="font-medium">Tutup buku tahun fiskal {ov.fiscalYear}</h2>
                {ov.closing ? (
                  <p className="flex items-center gap-1.5 text-sm text-emerald-700 dark:text-emerald-400">
                    <CheckCircle2 className="size-4" /> Ditutup pada{" "}
                    {formatDateTime(ov.closing.closedAt)}
                  </p>
                ) : (
                  <p className="text-sm text-muted-foreground">Belum ditutup.</p>
                )}
              </div>
              {ov.closing ? (
                <Button
                  variant="destructive"
                  disabled={act.isPending}
                  onClick={() => setConfirm({ kind: "reopen-year" })}
                >
                  Batalkan tutup buku
                </Button>
              ) : (
                <Button
                  disabled={!ov.canCloseYear || act.isPending}
                  onClick={() => setConfirm({ kind: "close-year" })}
                >
                  Tutup buku {ov.fiscalYear}
                </Button>
              )}
            </div>

            {ov.closing ? (
              <p className="text-sm text-muted-foreground">
                {ov.closing.netIncome >= 0 ? "Laba" : "Rugi"} bersih{" "}
                <span className="font-medium tabular-nums text-foreground">
                  {formatMoney(Math.abs(ov.closing.netIncome))}
                </span>{" "}
                dipindahkan ke Laba Ditahan.{" "}
                {ov.closing.entryId !== null ? (
                  <>
                    Jurnal penutup:{" "}
                    <Link
                      to={`/jurnal/${ov.closing.entryId}`}
                      className="font-mono underline-offset-2 hover:underline"
                    >
                      {ov.closing.number}
                    </Link>
                    .
                  </>
                ) : (
                  "Tahun ini tidak memiliki saldo pendapatan/beban, jadi tidak ada jurnal penutup."
                )}
              </p>
            ) : (
              <>
                {ov.blockers.length > 0 && (
                  <ul
                    className="list-disc space-y-0.5 pl-5 text-sm text-muted-foreground"
                    data-blockers
                  >
                    {ov.blockers.map((b) => (
                      <li key={b}>{b}</li>
                    ))}
                  </ul>
                )}
                <p className="text-sm text-muted-foreground">
                  Perkiraan {ov.previewNetIncome >= 0 ? "laba" : "rugi"} yang dipindahkan:{" "}
                  <span className="font-medium tabular-nums text-foreground" data-preview-net>
                    {formatMoney(Math.abs(ov.previewNetIncome))}
                  </span>{" "}
                  ({ov.previewAccounts} akun pendapatan/beban akan ditolkan).
                </p>
              </>
            )}
          </section>
        </>
      )}

      <AlertDialog open={confirm !== null} onOpenChange={(o) => !o && setConfirm(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{confirmTitle(confirm, fy)}</AlertDialogTitle>
            <AlertDialogDescription>{confirmText(confirm, fy)}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Batal</AlertDialogCancel>
            <Button
              data-confirm
              variant={confirm?.kind === "reopen-year" ? "destructive" : "default"}
              disabled={act.isPending}
              onClick={() => confirm && act.mutate(confirm)}
            >
              {act.isPending ? "Memproses..." : "Ya, lanjutkan"}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

/** Teks toast sesuai aksi (dibangun hanya untuk aksi yang berjalan). */
function successText(c: Confirm, fy: number | null): string {
  switch (c.kind) {
    case "close-month":
      return `Periode ${monthLong(monthKey(c.m))} ditutup`;
    case "reopen-month":
      return `Periode ${monthLong(monthKey(c.m))} dibuka kembali`;
    case "close-year":
      return `Tahun fiskal ${fy} ditutup buku`;
    case "reopen-year":
      return `Tutup buku tahun fiskal ${fy} dibatalkan`;
  }
}

function confirmTitle(c: Confirm | null, fy: number | null): string {
  if (!c) return "";
  switch (c.kind) {
    case "close-month":
      return `Tutup periode ${monthLong(monthKey(c.m))}?`;
    case "reopen-month":
      return `Buka kembali ${monthLong(monthKey(c.m))}?`;
    case "close-year":
      return `Tutup buku tahun fiskal ${fy}?`;
    case "reopen-year":
      return `Batalkan tutup buku ${fy}?`;
  }
}

function confirmText(c: Confirm | null, fy: number | null): string {
  if (!c) return "";
  switch (c.kind) {
    case "close-month":
      return "Jurnal pada periode ini tidak bisa dibuat, diubah, atau dihapus sampai periodenya dibuka kembali.";
    case "reopen-month":
      return "Jurnal pada periode ini bisa diubah lagi. Pembukaan tercatat di riwayat perubahan.";
    case "close-year":
      return `Aplikasi membuat satu jurnal penutup bertanggal akhir tahun fiskal ${fy}: akun pendapatan dan beban dinolkan dan labanya dipindahkan ke Laba Ditahan. Laporan Laba Rugi tahun ini tidak berubah.`;
    case "reopen-year":
      return "Jurnal penutup dihapus (isinya tersimpan di riwayat) dan saldo pendapatan/beban kembali seperti sebelum tutup buku. Periode bulanan tetap tertutup; buka kembali satu per satu bila perlu mengubah jurnal.";
  }
}
