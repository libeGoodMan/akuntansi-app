import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
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
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  ACCOUNT_TYPE_LABEL,
  api,
  errorMessage,
  type Account,
  type AccountType,
  type JournalDetail,
  type JournalLineInput,
} from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/money";

const BS_TYPES: AccountType[] = ["ASSET", "LIABILITY", "EQUITY"];

/** Halaman pemuat: tunggu akun, jurnal saldo awal (bila ada), dan profil perusahaan. */
export function OpeningPage() {
  const accounts = useQuery({ queryKey: ["accounts"], queryFn: api.listAccounts });
  const opening = useQuery({ queryKey: ["opening"], queryFn: api.getOpening });
  const company = useQuery({ queryKey: ["company"], queryFn: api.getCompany });

  const error = accounts.error ?? opening.error ?? company.error;
  if (error) return <p className="text-sm text-destructive">{errorMessage(error)}</p>;
  if (!accounts.data || opening.data === undefined || company.data === undefined) {
    return <p className="text-sm text-muted-foreground">Memuat...</p>;
  }

  const startMonth = company.data?.fiscalYearStartMonth ?? 1;
  const defaultDate = `${new Date().getFullYear()}-${String(startMonth).padStart(2, "0")}-01`;

  return (
    <OpeningForm
      key={opening.data ? `${opening.data.id}-${opening.data.updatedAt ?? ""}` : "none"}
      accounts={accounts.data}
      existing={opening.data}
      defaultDate={defaultDate}
    />
  );
}

function OpeningForm({
  accounts,
  existing,
  defaultDate,
}: {
  accounts: Account[];
  existing: JournalDetail | null;
  defaultDate: string;
}) {
  const queryClient = useQueryClient();

  // Akun yang diisi: akun neraca yang bisa dijurnal, kecuali akun penyeimbang (otomatis).
  const balanceRows = useMemo(
    () =>
      accounts
        .filter(
          (a) =>
            !a.isHeader &&
            a.isActive &&
            BS_TYPES.includes(a.type) &&
            a.systemRole !== "OPENING_EQUITY",
        )
        .sort((a, b) => a.code.localeCompare(b.code)),
    [accounts],
  );
  const equityAccount = accounts.find((a) => a.systemRole === "OPENING_EQUITY");

  const [date, setDate] = useState(existing?.date ?? defaultDate);
  // Nilai positif = di sisi saldo normal akun; negatif = sisi sebaliknya.
  const [amounts, setAmounts] = useState<Record<number, string>>(() => {
    const init: Record<number, string> = {};
    for (const l of existing?.lines ?? []) {
      const acc = accounts.find((a) => a.id === l.accountId);
      if (!acc || acc.systemRole === "OPENING_EQUITY") continue;
      const signed = l.debit - l.credit; // + = debit
      const amount = acc.normalBalance === "DEBIT" ? signed : -signed;
      if (amount !== 0) init[acc.id] = formatMoney(amount);
    }
    return init;
  });
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

  // Hitung debit - kredit dari isian; selisihnya otomatis menjadi modal awal.
  const calc = useMemo(() => {
    let net = 0; // debit - kredit
    let invalid: string | null = null;
    const lines: JournalLineInput[] = [];
    for (const a of balanceRows) {
      const text = amounts[a.id] ?? "";
      if (!text.trim()) continue;
      const cents = parseMoney(text, { allowNegative: true });
      if (cents === null) {
        invalid ??= `${a.code} ${a.name}: nominal tidak valid`;
        continue;
      }
      if (cents === 0) continue;
      const signed = a.normalBalance === "DEBIT" ? cents : -cents; // debit - kredit
      net += signed;
      lines.push({
        accountId: a.id,
        debit: signed > 0 ? signed : 0,
        credit: signed < 0 ? -signed : 0,
        memo: null,
      });
    }
    return { net, lines, invalid };
  }, [amounts, balanceRows]);

  // Debit melebihi kredit (aset > kewajiban) -> selisihnya masuk KREDIT di Modal Awal (positif).
  // Negatif berarti kewajiban melebihi aset (defisit modal).
  const equityAuto = calc.net;

  const save = useMutation({
    mutationFn: () => api.saveOpening({ date, lines: calc.lines }),
    onSuccess: async () => {
      await queryClient.invalidateQueries();
      toast.success("Saldo awal disimpan");
    },
    onError: (e) => setError(errorMessage(e)),
  });

  const remove = useMutation({
    mutationFn: api.deleteOpening,
    onSuccess: async () => {
      setConfirmDelete(false);
      await queryClient.invalidateQueries();
      toast.success("Saldo awal dihapus");
    },
    onError: (e) => {
      setConfirmDelete(false);
      setError(errorMessage(e));
    },
  });

  function onSave() {
    setError(null);
    if (calc.invalid) return setError(calc.invalid);
    if (calc.lines.length === 0) return setError("Isi saldo minimal satu akun.");
    save.mutate();
  }

  const groups = BS_TYPES.map((t) => ({
    type: t,
    accounts: balanceRows.filter((a) => a.type === t),
  })).filter((g) => g.accounts.length > 0);

  return (
    <div className="mx-auto max-w-3xl space-y-5">
      <div className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Saldo Awal</h1>
        <p className="text-sm text-muted-foreground">
          Isi saldo akun neraca saat mulai memakai aplikasi. Selisihnya otomatis masuk ke{" "}
          {equityAccount ? `${equityAccount.code} ${equityAccount.name}` : "akun modal"}.
          {existing && <> Nomor jurnal: {existing.number}.</>}
        </p>
      </div>

      <section className="flex items-end gap-4 rounded-xl border p-5">
        <div className="space-y-1.5">
          <Label htmlFor="o-date">Tanggal mulai pembukuan</Label>
          <Input
            id="o-date"
            type="date"
            className="w-44"
            value={date}
            onChange={(e) => setDate(e.target.value)}
          />
        </div>
        <p className="pb-2 text-xs text-muted-foreground">
          Isi saldo di sisi normal akun (aset di debit, kewajiban di kredit). Angka negatif membalik
          sisinya.
        </p>
      </section>

      {groups.map((g) => (
        <section key={g.type} className="rounded-xl border">
          <h2 className="rounded-t-xl bg-muted/60 px-4 py-2.5 text-xs font-medium text-muted-foreground">
            {ACCOUNT_TYPE_LABEL[g.type]}
          </h2>
          <ul>
            {g.accounts.map((a) => (
              <li key={a.id} className="flex items-center gap-4 border-t px-4 py-2 text-sm">
                <span className="w-20 font-mono text-xs text-muted-foreground">{a.code}</span>
                <span className="flex-1">
                  {a.name}
                  {a.normalBalance !== (a.type === "ASSET" ? "DEBIT" : "CREDIT") && (
                    <span className="ml-2 text-xs text-muted-foreground">
                      (saldo normal {a.normalBalance === "DEBIT" ? "debit" : "kredit"})
                    </span>
                  )}
                </span>
                <MoneyInput
                  allowNegative
                  className="w-48"
                  data-account={a.code}
                  value={amounts[a.id] ?? ""}
                  onValueChange={(v) => setAmounts((s) => ({ ...s, [a.id]: v }))}
                />
              </li>
            ))}
          </ul>
        </section>
      ))}

      <section className="flex items-center justify-between rounded-xl border bg-muted/30 px-4 py-3 text-sm">
        <span className="font-medium">
          {equityAccount ? `${equityAccount.name} (otomatis)` : "Modal awal (otomatis)"}
        </span>
        <span className="tabular-nums" data-equity-auto>
          {formatMoney(equityAuto)}
          {equityAuto < 0 && <span className="ml-2 text-xs text-destructive">defisit</span>}
        </span>
      </section>

      {error && (
        <p
          role="alert"
          className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
        >
          {error}
        </p>
      )}

      <div className="flex items-center justify-between">
        <div>
          {existing && (
            <Button
              variant="destructive"
              disabled={save.isPending || remove.isPending}
              onClick={() => setConfirmDelete(true)}
            >
              Hapus saldo awal
            </Button>
          )}
        </div>
        <Button disabled={save.isPending || remove.isPending} onClick={onSave}>
          {save.isPending ? "Menyimpan..." : existing ? "Simpan perubahan" : "Simpan saldo awal"}
        </Button>
      </div>

      <AlertDialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Hapus saldo awal?</AlertDialogTitle>
            <AlertDialogDescription>
              Semua saldo awal dihapus dan laporan langsung berubah. Isinya tetap tersimpan di
              riwayat perubahan.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Batal</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={remove.isPending}
              onClick={() => remove.mutate()}
            >
              Hapus
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
