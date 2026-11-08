import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import { ArrowLeft, Plus, Trash2 } from "lucide-react";
import { AccountPicker } from "@/components/AccountPicker";
import { ExportButtons } from "@/components/ExportButtons";
import { MoneyInput } from "@/components/MoneyInput";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  api,
  errorMessage,
  type Account,
  type JournalDetail,
  type JournalInput,
  type JournalLineInput,
} from "@/lib/api";
import { formatDate, todayISO } from "@/lib/dates";
import { formatMoney, parseMoney } from "@/lib/money";
import { cn } from "@/lib/utils";
import { HistoryPanel } from "./HistoryPanel";

interface Row {
  key: number;
  accountId: number | null;
  debit: string;
  credit: string;
  memo: string;
}

const MIN_ROWS = 2;

/** Halaman pemuat: jurnal baru (/jurnal/baru) atau jurnal yang sudah ada (/jurnal/:id). */
export function JournalFormPage() {
  const { id } = useParams();
  const journalId = id ? Number(id) : null;

  const accounts = useQuery({ queryKey: ["accounts"], queryFn: api.listAccounts });
  const journal = useQuery({
    queryKey: ["journal", journalId],
    queryFn: () => api.getJournal(journalId!),
    enabled: journalId !== null,
  });

  const error = accounts.error ?? journal.error;
  if (error) {
    return (
      <div className="space-y-3">
        <BackLink />
        <p className="text-sm text-destructive">{errorMessage(error)}</p>
      </div>
    );
  }
  if (!accounts.data || (journalId !== null && !journal.data)) {
    return <p className="text-sm text-muted-foreground">Memuat...</p>;
  }

  return (
    <JournalForm
      // Pasang ulang form setiap data jurnal berubah (mis. setelah disimpan).
      key={
        journal.data
          ? `${journal.data.id}-${journal.data.updatedAt ?? ""}-${journal.data.status}`
          : "new"
      }
      initial={journal.data ?? null}
      accounts={accounts.data}
    />
  );
}

function BackLink() {
  return (
    <Link
      to="/jurnal"
      className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
    >
      <ArrowLeft className="size-4" /> Daftar jurnal
    </Link>
  );
}

let rowSeq = 0;
const newRow = (): Row => ({ key: ++rowSeq, accountId: null, debit: "", credit: "", memo: "" });

function rowsFrom(j: JournalDetail | null): Row[] {
  if (!j) return Array.from({ length: MIN_ROWS }, newRow);
  return j.lines.map((l) => ({
    key: ++rowSeq,
    accountId: l.accountId,
    debit: l.debit ? formatMoney(l.debit) : "",
    credit: l.credit ? formatMoney(l.credit) : "",
    memo: l.memo ?? "",
  }));
}

const isBlank = (r: Row) =>
  r.accountId === null && !r.debit.trim() && !r.credit.trim() && !r.memo.trim();

function JournalForm({
  initial,
  accounts,
}: {
  initial: JournalDetail | null;
  accounts: Account[];
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const isNew = initial === null;
  const isPosted = initial?.status === "POSTED";
  const readOnly = initial !== null && initial.type !== "GENERAL";

  const [date, setDate] = useState(initial?.date ?? todayISO());
  const [description, setDescription] = useState(initial?.description ?? "");
  const [reference, setReference] = useState(initial?.reference ?? "");
  const [rows, setRows] = useState<Row[]>(() => rowsFrom(initial));
  const [confirm, setConfirm] = useState<"save-posted" | "delete" | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  const postable = useMemo(() => accounts.filter((a) => !a.isHeader && a.isActive), [accounts]);

  // ----- total & selisih (dihitung dari teks, hanya untuk tampilan) -----
  const totals = useMemo(() => {
    let debit = 0;
    let credit = 0;
    for (const r of rows) {
      debit += parseMoney(r.debit) ?? 0;
      credit += parseMoney(r.credit) ?? 0;
    }
    return { debit, credit, diff: debit - credit };
  }, [rows]);
  const filledRows = rows.filter((r) => !isBlank(r)).length;
  const balanced = totals.diff === 0 && totals.debit > 0 && filledRows >= 2;

  // ----- fokus & baris -----
  const pendingFocus = useRef<string | null>(null);
  useEffect(() => {
    if (pendingFocus.current) {
      document.querySelector<HTMLElement>(`[data-cell="${pendingFocus.current}"]`)?.focus();
      pendingFocus.current = null;
    }
  }, [rows]);

  const updateRow = (key: number, patch: Partial<Row>) =>
    setRows((rs) => rs.map((r) => (r.key === key ? { ...r, ...patch } : r)));

  function addRow(focusIndex?: number) {
    setRows((rs) => [...rs, newRow()]);
    if (focusIndex !== undefined) pendingFocus.current = `r${focusIndex}-account`;
  }

  function removeRow(key: number) {
    setRows((rs) => (rs.length <= 1 ? rs : rs.filter((r) => r.key !== key)));
  }

  /** Enter di baris terakhir menambah baris; di baris lain pindah ke akun baris berikutnya. */
  function onRowEnter(index: number) {
    if (index === rows.length - 1) addRow(index + 1);
    else document.querySelector<HTMLElement>(`[data-cell="r${index + 1}-account"]`)?.focus();
  }

  /** Isi baris dengan selisih supaya seimbang. */
  function fillDifference(row: Row) {
    const others = rows.filter((r) => r.key !== row.key);
    const d = others.reduce((s, r) => s + (parseMoney(r.debit) ?? 0), 0);
    const c = others.reduce((s, r) => s + (parseMoney(r.credit) ?? 0), 0);
    if (d === c) return;
    if (d > c) updateRow(row.key, { debit: "", credit: formatMoney(d - c) });
    else updateRow(row.key, { debit: formatMoney(c - d), credit: "" });
  }

  // ----- bentuk input untuk Rust -----
  function buildInput(): JournalInput | null {
    setFormError(null);
    if (!description.trim()) {
      setFormError("Keterangan wajib diisi.");
      return null;
    }
    const lines: JournalLineInput[] = [];
    for (const [i, r] of rows.entries()) {
      if (isBlank(r)) continue;
      const n = i + 1;
      if (r.accountId === null) return fail(`Baris ${n}: pilih akun.`);
      const d = r.debit.trim() ? parseMoney(r.debit) : 0;
      const c = r.credit.trim() ? parseMoney(r.credit) : 0;
      if (d === null || c === null)
        return fail(`Baris ${n}: nominal tidak valid (contoh 1.250.000,50).`);
      if (d > 0 && c > 0) return fail(`Baris ${n}: isi debit atau kredit, jangan keduanya.`);
      if (d === 0 && c === 0) return fail(`Baris ${n}: isi nominal debit atau kredit.`);
      lines.push({ accountId: r.accountId, debit: d, credit: c, memo: r.memo.trim() || null });
    }
    return { date, description, reference: reference.trim() || null, lines };
  }

  function fail(msg: string): null {
    setFormError(msg);
    return null;
  }

  // ----- aksi -----
  const done = async (j: JournalDetail, message: string) => {
    await queryClient.invalidateQueries({ queryKey: ["journals"] });
    await queryClient.invalidateQueries({ queryKey: ["journal", j.id] });
    await queryClient.invalidateQueries({ queryKey: ["journal-history", j.id] });
    await queryClient.invalidateQueries({ queryKey: ["accounts"] }); // status "terpakai" berubah
    toast.success(message);
  };

  const save = useMutation({
    mutationFn: async ({ input, post }: { input: JournalInput; post: boolean }) =>
      initial ? api.updateJournal(initial.id, input, post) : api.createJournal(input, post),
    onSuccess: async (j, { post }) => {
      await done(
        j,
        post && j.status === "POSTED"
          ? `Jurnal ${j.number} diposting`
          : `Jurnal ${j.number} disimpan`,
      );
      if (isNew) navigate(post ? "/jurnal" : `/jurnal/${j.id}`, { replace: true });
      else if (post) navigate("/jurnal");
    },
    onError: (e) => setFormError(errorMessage(e)),
  });

  const remove = useMutation({
    mutationFn: () => api.deleteJournal(initial!.id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["journals"] });
      await queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success(`Jurnal ${initial!.number} dihapus`);
      navigate("/jurnal", { replace: true });
    },
    onError: (e) => {
      setConfirm(null);
      setFormError(errorMessage(e));
    },
  });

  const busy = save.isPending || remove.isPending;

  function submit(post: boolean) {
    const input = buildInput();
    if (!input) return;
    save.mutate({ input, post });
  }

  /** Simpan (draft) atau, untuk jurnal yang sudah diposting, minta konfirmasi dulu. */
  function onSave() {
    if (isPosted) {
      if (buildInput()) setConfirm("save-posted");
    } else submit(false);
  }

  // Ctrl+S = simpan, Ctrl+Enter = posting.
  const handlers = useRef({ onSave, submit, balanced, busy });
  useEffect(() => {
    handlers.current = { onSave, submit, balanced, busy };
  });
  useEffect(() => {
    if (readOnly) return;
    const onKey = (e: KeyboardEvent) => {
      const h = handlers.current;
      if (h.busy) return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        h.onSave();
      } else if ((e.ctrlKey || e.metaKey) && e.key === "Enter" && !isPosted) {
        e.preventDefault();
        if (h.balanced) h.submit(true);
        else setFormError("Jurnal belum seimbang, belum bisa diposting.");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [readOnly, isPosted]);

  // ----- tampilan -----
  const title = isNew ? "Jurnal baru" : initial.number;

  return (
    <div className="mx-auto max-w-5xl space-y-5">
      <BackLink />

      <div className="flex items-start justify-between gap-4">
        <div className="space-y-1">
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
            {initial && (
              <Badge variant={isPosted ? "secondary" : "outline"}>
                {isPosted ? "Diposting" : "Draft"}
              </Badge>
            )}
          </div>
          {isPosted && !readOnly && (
            <p className="text-sm text-muted-foreground">
              Jurnal ini sudah diposting. Perubahan langsung memengaruhi laporan dan tercatat di
              riwayat.
            </p>
          )}
          {readOnly && (
            <p className="text-sm text-muted-foreground">
              Jurnal {initial.type === "OPENING" ? "saldo awal" : "penutup"} tidak bisa diubah di
              sini.{" "}
              {initial.type === "OPENING" && (
                <Link to="/saldo-awal" className="underline">
                  Buka layar Saldo Awal
                </Link>
              )}
            </p>
          )}
        </div>
        {!isNew && (
          <div className="flex items-center gap-2">
            <ExportButtons
              formats={["pdf"]}
              name={`bukti-${initial.number}`}
              request={{ kind: "voucher", id: initial.id }}
            />
            {!readOnly && (
              <Button variant="destructive" disabled={busy} onClick={() => setConfirm("delete")}>
                <Trash2 className="size-4" /> Hapus
              </Button>
            )}
          </div>
        )}
      </div>

      <section className="grid gap-4 rounded-xl border p-5 sm:grid-cols-[10rem_1fr_14rem]">
        <div className="space-y-1.5">
          <Label htmlFor="j-date">Tanggal</Label>
          <Input
            id="j-date"
            type="date"
            value={date}
            disabled={readOnly}
            onChange={(e) => setDate(e.target.value)}
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="j-desc">Keterangan</Label>
          <Input
            id="j-desc"
            value={description}
            disabled={readOnly}
            maxLength={300}
            placeholder="Mis. Penjualan tunai ke PT Maju"
            onChange={(e) => setDescription(e.target.value)}
            autoFocus={isNew}
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="j-ref">Referensi (no. faktur/bukti)</Label>
          <Input
            id="j-ref"
            value={reference}
            disabled={readOnly}
            maxLength={60}
            onChange={(e) => setReference(e.target.value)}
          />
        </div>
      </section>

      <section className="rounded-xl border">
        <table className="w-full text-sm">
          <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
            <tr>
              <th className="w-8 rounded-tl-xl px-3 py-2.5 font-medium">#</th>
              <th className="px-2 py-2.5 font-medium">Akun</th>
              <th className="w-44 px-2 py-2.5 text-right font-medium">Debit</th>
              <th className="w-44 px-2 py-2.5 text-right font-medium">Kredit</th>
              <th className="w-56 px-2 py-2.5 font-medium">Catatan baris</th>
              <th className="w-10 rounded-tr-xl px-2 py-2.5" />
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={r.key} className="border-t align-top">
                <td className="px-3 py-3 text-xs text-muted-foreground">{i + 1}</td>
                <td className="px-2 py-2">
                  <AccountPicker
                    cell={`r${i}-account`}
                    options={postable}
                    all={accounts}
                    value={r.accountId}
                    disabled={readOnly}
                    onChange={(accountId) => updateRow(r.key, { accountId })}
                  />
                </td>
                <td className="px-2 py-2">
                  <MoneyInput
                    data-cell={`r${i}-debit`}
                    value={r.debit}
                    disabled={readOnly}
                    onValueChange={(v) =>
                      updateRow(r.key, { debit: v, credit: v.trim() ? "" : r.credit })
                    }
                    onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), onRowEnter(i))}
                  />
                </td>
                <td className="px-2 py-2">
                  <MoneyInput
                    data-cell={`r${i}-credit`}
                    value={r.credit}
                    disabled={readOnly}
                    onValueChange={(v) =>
                      updateRow(r.key, { credit: v, debit: v.trim() ? "" : r.debit })
                    }
                    onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), onRowEnter(i))}
                  />
                </td>
                <td className="px-2 py-2">
                  <Input
                    data-cell={`r${i}-memo`}
                    value={r.memo}
                    disabled={readOnly}
                    onChange={(e) => updateRow(r.key, { memo: e.target.value })}
                    onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), onRowEnter(i))}
                  />
                </td>
                <td className="px-1 py-2">
                  {!readOnly && (
                    <div className="flex">
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        title="Isi dengan selisih"
                        tabIndex={-1}
                        disabled={totals.diff === 0 && isBlank(r)}
                        onClick={() => fillDifference(r)}
                      >
                        <span className="text-xs font-semibold">=</span>
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        title="Hapus baris"
                        tabIndex={-1}
                        disabled={rows.length <= 1}
                        onClick={() => removeRow(r.key)}
                      >
                        <Trash2 className="size-4" />
                      </Button>
                    </div>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr className="border-t bg-muted/30 font-medium">
              <td />
              <td className="px-2 py-3">
                {!readOnly && (
                  <Button variant="ghost" size="sm" onClick={() => addRow(rows.length)}>
                    <Plus className="size-4" /> Tambah baris
                  </Button>
                )}
              </td>
              <td className="px-4 py-3 text-right tabular-nums" data-total="debit">
                {formatMoney(totals.debit)}
              </td>
              <td className="px-4 py-3 text-right tabular-nums" data-total="credit">
                {formatMoney(totals.credit)}
              </td>
              <td colSpan={2} className="px-3 py-3" data-balance={balanced ? "ok" : "off"}>
                {filledRows === 0 ? (
                  <span className="text-xs text-muted-foreground">Belum ada baris terisi</span>
                ) : totals.diff === 0 && totals.debit > 0 ? (
                  <span className="text-xs text-emerald-600 dark:text-emerald-400">
                    {filledRows < 2 ? "Minimal 2 baris" : "Seimbang"}
                  </span>
                ) : (
                  <span className="text-xs text-destructive">
                    Selisih {formatMoney(Math.abs(totals.diff))}
                  </span>
                )}
              </td>
            </tr>
          </tfoot>
        </table>
      </section>

      {formError && (
        <p
          role="alert"
          className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
        >
          {formError}
        </p>
      )}

      {!readOnly && (
        <div className="flex items-center justify-between gap-4">
          <p className="text-xs text-muted-foreground">
            Enter di baris terakhir menambah baris · Ctrl+S simpan
            {isPosted ? "" : " · Ctrl+Enter posting"}
          </p>
          <div className="flex gap-2">
            {isPosted ? (
              <Button
                disabled={busy || !balanced}
                onClick={onSave}
                title={balanced ? undefined : "Jurnal harus seimbang"}
              >
                Simpan perubahan
              </Button>
            ) : (
              <>
                <Button variant="outline" disabled={busy} onClick={onSave}>
                  Simpan draft
                </Button>
                <Button
                  disabled={busy || !balanced}
                  onClick={() => submit(true)}
                  title={balanced ? undefined : "Jurnal harus seimbang"}
                >
                  Posting
                </Button>
              </>
            )}
          </div>
        </div>
      )}

      {initial && <HistoryPanel journalId={initial.id} />}

      <AlertDialog open={confirm !== null} onOpenChange={(o) => !o && setConfirm(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {confirm === "delete"
                ? `Hapus jurnal ${initial?.number}?`
                : "Simpan perubahan jurnal yang sudah diposting?"}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {confirm === "delete"
                ? `${initial?.description} (${initial ? formatDate(initial.date) : ""}) akan dihapus dan laporan langsung berubah. Nomornya tidak dipakai ulang; isinya tetap tersimpan di riwayat perubahan.`
                : "Laporan keuangan langsung mengikuti perubahan ini. Isi sebelum dan sesudahnya dicatat di riwayat."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Batal</AlertDialogCancel>
            <Button
              variant={confirm === "delete" ? "destructive" : "default"}
              disabled={busy}
              className={cn(confirm !== "delete" && "")}
              onClick={() => {
                if (confirm === "delete") remove.mutate();
                else {
                  setConfirm(null);
                  const input = buildInput();
                  if (input) save.mutate({ input, post: false });
                }
              }}
            >
              {confirm === "delete" ? "Hapus" : "Simpan perubahan"}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
