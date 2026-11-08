import { useQuery } from "@tanstack/react-query";
import { api, errorMessage, type HistoryItem, type JournalDetail } from "@/lib/api";
import { formatDate, formatDateTime } from "@/lib/dates";
import { formatMoney } from "@/lib/money";

const ACTION_LABEL: Record<string, string> = {
  CREATE: "Dibuat",
  POST: "Diposting",
  UPDATE: "Diubah",
  DELETE: "Dihapus",
  OPENING_SAVE: "Saldo awal disimpan",
};

const total = (j: JournalDetail) => j.lines.reduce((s, l) => s + l.debit, 0);
const signature = (j: JournalDetail) =>
  JSON.stringify(j.lines.map((l) => [l.accountId, l.debit, l.credit, l.memo ?? ""]));

/** Ringkasan perubahan dalam kalimat pendek, dihitung dari isi sebelum/sesudah di audit log. */
function summarize(h: HistoryItem): string[] {
  const d = h.detail;
  if (!d) return [];
  const { before, after } = d;
  if (h.action === "UPDATE" && before && after) {
    const out: string[] = [];
    if (before.date !== after.date)
      out.push(`Tanggal ${formatDate(before.date)} → ${formatDate(after.date)}`);
    if (before.description !== after.description) out.push("Keterangan diubah");
    if ((before.reference ?? "") !== (after.reference ?? "")) out.push("Referensi diubah");
    if (total(before) !== total(after))
      out.push(`Total ${formatMoney(total(before))} → ${formatMoney(total(after))}`);
    else if (signature(before) !== signature(after)) out.push("Isi baris diubah (total sama)");
    if (before.lines.length !== after.lines.length)
      out.push(`Jumlah baris ${before.lines.length} → ${after.lines.length}`);
    return out.length ? out : ["Disimpan tanpa perubahan isi"];
  }
  if (h.action === "CREATE" && after)
    return [`${after.lines.length} baris, total ${formatMoney(total(after))}`];
  if (h.action === "DELETE" && before)
    return [`${before.number} · total ${formatMoney(total(before))}`];
  return [];
}

export function HistoryPanel({ journalId }: { journalId: number }) {
  const { data, error } = useQuery({
    queryKey: ["journal-history", journalId],
    queryFn: () => api.journalHistory(journalId),
  });

  return (
    <section className="space-y-3 rounded-xl border p-5">
      <h2 className="text-sm font-medium">Riwayat perubahan</h2>
      {error && <p className="text-sm text-destructive">{errorMessage(error)}</p>}
      {data && data.length === 0 && (
        <p className="text-sm text-muted-foreground">Belum ada riwayat.</p>
      )}
      <ol className="space-y-3">
        {data?.map((h) => (
          <li key={h.id} className="border-l-2 pl-3 text-sm">
            <div className="flex items-baseline gap-2">
              <span className="font-medium">{ACTION_LABEL[h.action] ?? h.action}</span>
              <span className="text-xs text-muted-foreground">{formatDateTime(h.at)}</span>
            </div>
            {summarize(h).map((s) => (
              <div key={s} className="text-xs text-muted-foreground">
                {s}
              </div>
            ))}
          </li>
        ))}
      </ol>
    </section>
  );
}
