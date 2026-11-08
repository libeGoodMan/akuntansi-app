import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useNavigate } from "react-router-dom";
import { BookOpen, Eye, EyeOff, Pencil, Plus, Search, Trash2 } from "lucide-react";
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
import { ACCOUNT_TYPE_LABEL, api, errorMessage, type Account } from "@/lib/api";
import { cn } from "@/lib/utils";
import { AccountDialog, type DialogMode } from "./AccountDialog";
import { buildRows } from "./tree";

export function AccountsPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const {
    data: accounts = [],
    error,
    isLoading,
  } = useQuery({
    queryKey: ["accounts"],
    queryFn: api.listAccounts,
  });

  const [search, setSearch] = useState("");
  const [showInactive, setShowInactive] = useState(false);
  const [dialog, setDialog] = useState<DialogMode | null>(null);
  const [dialogKey, setDialogKey] = useState(0);
  const [toDelete, setToDelete] = useState<Account | null>(null);

  const rows = useMemo(() => {
    const q = search.trim().toLowerCase();
    return buildRows(
      accounts,
      (a) =>
        (showInactive || a.isActive) &&
        (q === "" || a.code.toLowerCase().includes(q) || a.name.toLowerCase().includes(q)),
    );
  }, [accounts, search, showInactive]);

  const hiddenInactive = accounts.filter((a) => !a.isActive).length;

  function open(mode: DialogMode) {
    setDialogKey((k) => k + 1);
    setDialog(mode);
  }

  const toggleActive = useMutation({
    mutationFn: (a: Account) => api.setAccountActive(a.id, !a.isActive),
    onSuccess: async (a) => {
      await queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success(`Akun ${a.code} ${a.isActive ? "diaktifkan" : "dinonaktifkan"}`);
    },
    onError: (e) => toast.error(errorMessage(e)),
  });

  const remove = useMutation({
    mutationFn: (a: Account) => api.deleteAccount(a.id),
    onSuccess: async (_, a) => {
      await queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success(`Akun ${a.code} dihapus`);
      setToDelete(null);
    },
    onError: (e) => {
      toast.error(errorMessage(e));
      setToDelete(null);
    },
  });

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Chart of Accounts</h1>
          <p className="text-sm text-muted-foreground">
            {accounts.length} akun. Akun yang sudah dipakai di jurnal tidak bisa dihapus, hanya
            dinonaktifkan.
          </p>
        </div>
        <Button onClick={() => open({ kind: "create", parent: null })}>
          <Plus className="size-4" /> Tambah akun
        </Button>
      </div>

      <div className="flex items-center gap-4">
        <div className="relative max-w-sm flex-1">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="pl-8"
            placeholder="Cari kode atau nama akun"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={showInactive}
            onChange={(e) => setShowInactive(e.target.checked)}
          />
          Tampilkan nonaktif{hiddenInactive > 0 && ` (${hiddenInactive})`}
        </label>
      </div>

      {error && <p className="text-sm text-destructive">{errorMessage(error)}</p>}

      <div className="overflow-hidden rounded-xl border">
        <table className="w-full text-sm">
          <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
            <tr>
              <th className="w-32 px-4 py-2.5 font-medium">Kode</th>
              <th className="px-4 py-2.5 font-medium">Nama akun</th>
              <th className="w-28 px-4 py-2.5 font-medium">Jenis</th>
              <th className="w-28 px-4 py-2.5 font-medium">Saldo normal</th>
              <th className="w-44 px-4 py-2.5 text-right font-medium">Aksi</th>
            </tr>
          </thead>
          <tbody>
            {isLoading && (
              <tr>
                <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                  Memuat...
                </td>
              </tr>
            )}
            {!isLoading && rows.length === 0 && (
              <tr>
                <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                  {search ? "Tidak ada akun yang cocok." : "Belum ada akun."}
                </td>
              </tr>
            )}
            {rows.map(({ account: a, depth, hasChildren }) => (
              <tr key={a.id} className={cn("border-t", !a.isActive && "text-muted-foreground")}>
                <td className="px-4 py-2 font-mono text-xs">{a.code}</td>
                <td className="px-4 py-2">
                  <span
                    style={{ paddingLeft: `${depth * 1.25}rem` }}
                    className={cn("inline-flex items-center gap-2", a.isHeader && "font-semibold")}
                  >
                    {a.name}
                    {a.isSystem && <Badge variant="secondary">Sistem</Badge>}
                    {a.isCash && <Badge variant="outline">Kas/Bank</Badge>}
                    {!a.isActive && <Badge variant="outline">Nonaktif</Badge>}
                  </span>
                </td>
                <td className="px-4 py-2">{ACCOUNT_TYPE_LABEL[a.type]}</td>
                <td className="px-4 py-2">
                  {a.isHeader ? "-" : a.normalBalance === "DEBIT" ? "Debit" : "Kredit"}
                </td>
                <td className="px-4 py-1.5">
                  <div className="flex justify-end gap-0.5">
                    {a.isHeader && a.isActive && (
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        title="Tambah akun anak"
                        onClick={() => open({ kind: "create", parent: a })}
                      >
                        <Plus className="size-4" />
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      title="Buku besar"
                      onClick={() => navigate(`/buku-besar?akun=${a.id}`)}
                    >
                      <BookOpen className="size-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      title="Ubah"
                      onClick={() => open({ kind: "edit", account: a })}
                    >
                      <Pencil className="size-4" />
                    </Button>
                    {!a.isSystem && (
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        title={a.isActive ? "Nonaktifkan" : "Aktifkan"}
                        disabled={toggleActive.isPending}
                        onClick={() => toggleActive.mutate(a)}
                      >
                        {a.isActive ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                      </Button>
                    )}
                    {!a.isSystem && !a.inUse && !hasChildren && (
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        title="Hapus"
                        onClick={() => setToDelete(a)}
                      >
                        <Trash2 className="size-4" />
                      </Button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {dialog && (
        <AccountDialog
          key={dialogKey}
          mode={dialog}
          accounts={accounts}
          onClose={() => setDialog(null)}
        />
      )}

      <AlertDialog open={toDelete !== null} onOpenChange={(o) => !o && setToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Hapus akun {toDelete?.code}?</AlertDialogTitle>
            <AlertDialogDescription>
              {toDelete?.name} akan dihapus permanen. Tindakan ini tidak bisa dibatalkan, tetapi
              tercatat di riwayat perubahan.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Batal</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={remove.isPending}
              onClick={() => toDelete && remove.mutate(toDelete)}
            >
              Hapus
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
