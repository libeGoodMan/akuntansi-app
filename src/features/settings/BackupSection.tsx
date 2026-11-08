import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { DatabaseBackup, FolderOpen, Trash2 } from "lucide-react";
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
import { api, errorMessage, type BackupCheck, type BackupInfo, type BackupReason } from "@/lib/api";
import { formatDate } from "@/lib/dates";

const REASON_LABEL: Record<BackupReason, string> = {
  keluar: "Otomatis (saat keluar)",
  "pra-migrasi": "Otomatis (sebelum migrasi data)",
  "pra-update": "Otomatis (sebelum update)",
  "pra-restore": "Otomatis (sebelum restore)",
  manual: "Manual",
};

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024)
    return `${(bytes / 1024 / 1024).toLocaleString("id-ID", { maximumFractionDigits: 1 })} MB`;
  return `${Math.max(1, Math.round(bytes / 1024)).toLocaleString("id-ID")} KB`;
}

interface Pending {
  path: string;
  label: string;
  check: BackupCheck;
}

/** Backup dan restore database: daftar backup otomatis, backup manual, dan pemulihan dari berkas. */
export function BackupSection() {
  const queryClient = useQueryClient();
  const [pending, setPending] = useState<Pending | null>(null);
  const [error, setError] = useState<string | null>(null);

  const list = useQuery({ queryKey: ["backups"], queryFn: api.listBackups });

  const fail = (e: unknown) => setError(errorMessage(e));

  const backupNow = useMutation({
    mutationFn: api.createBackup,
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: ["backups"] });
      toast.success("Backup dibuat");
    },
    onError: fail,
  });

  const backupAs = useMutation({
    mutationFn: async () => {
      const path = await save({
        defaultPath: `akuntansi-backup-${new Date().toISOString().slice(0, 10)}.db`,
        filters: [{ name: "Backup Akuntansi", extensions: ["db"] }],
      });
      if (!path) return null;
      return api.backupToPath(path.toLowerCase().endsWith(".db") ? path : `${path}.db`);
    },
    onSuccess: (path) => {
      setError(null);
      if (path) toast.success(`Backup tersimpan: ${path}`);
    },
    onError: fail,
  });

  /** Periksa berkas dulu, lalu minta konfirmasi dengan ringkasan isinya. */
  const prepare = useMutation({
    mutationFn: async (target: { path: string; label: string } | null) => {
      let path = target?.path ?? null;
      let label = target?.label ?? "";
      if (!path) {
        const picked = await open({
          multiple: false,
          filters: [{ name: "Backup Akuntansi", extensions: ["db"] }],
        });
        if (!picked || Array.isArray(picked)) return null;
        path = picked;
        label = picked;
      }
      const check = await api.checkBackup(path);
      return { path, label, check } satisfies Pending;
    },
    onSuccess: (p) => {
      setError(null);
      if (p) setPending(p);
    },
    onError: fail,
  });

  const restore = useMutation({
    mutationFn: (path: string) => api.restoreBackup(path),
    onSuccess: async () => {
      setError(null);
      setPending(null);
      await queryClient.invalidateQueries(); // semua data berasal dari database yang baru
      toast.success("Data dipulihkan dari backup");
    },
    onError: (e) => {
      setPending(null);
      fail(e);
    },
  });

  const remove = useMutation({
    mutationFn: (b: BackupInfo) => api.deleteBackup(b.fileName),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["backups"] });
      toast.success("Backup dihapus");
    },
    onError: fail,
  });

  const busy = backupNow.isPending || backupAs.isPending || prepare.isPending || restore.isPending;

  return (
    <section className="space-y-4 rounded-xl border p-5" data-backup-section>
      <div>
        <h2 className="text-sm font-medium">Backup dan restore</h2>
        <p className="text-xs text-muted-foreground">
          Salinan otomatis dibuat saat aplikasi ditutup (jika ada perubahan) dan sebelum update yang
          mengubah database. Simpan juga satu salinan di luar komputer ini.
        </p>
      </div>

      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          disabled={busy}
          onClick={() => backupNow.mutate()}
          data-action="backup-now"
        >
          <DatabaseBackup className="size-4" /> Backup sekarang
        </Button>
        <Button size="sm" variant="outline" disabled={busy} onClick={() => backupAs.mutate()}>
          Simpan backup ke lokasi lain...
        </Button>
        <Button size="sm" variant="outline" disabled={busy} onClick={() => prepare.mutate(null)}>
          <FolderOpen className="size-4" /> Pulihkan dari berkas...
        </Button>
      </div>

      {error && (
        <p
          role="alert"
          className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
        >
          {error}
        </p>
      )}

      <div>
        {list.data && (
          <p className="mb-2 text-xs text-muted-foreground">
            Folder backup: <span className="font-mono break-all">{list.data.dir}</span>
          </p>
        )}
        <div className="rounded-lg border">
          <table className="w-full text-sm">
            <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
              <tr>
                <th className="rounded-tl-lg px-3 py-2 font-medium">Waktu</th>
                <th className="px-3 py-2 font-medium">Jenis</th>
                <th className="w-24 px-3 py-2 text-right font-medium">Ukuran</th>
                <th className="w-44 rounded-tr-lg px-3 py-2 text-right font-medium">Aksi</th>
              </tr>
            </thead>
            <tbody>
              {list.data?.backups.length === 0 && (
                <tr>
                  <td colSpan={4} className="px-3 py-6 text-center text-muted-foreground">
                    Belum ada backup.
                  </td>
                </tr>
              )}
              {list.data?.backups.map((b) => (
                <tr key={b.fileName} className="border-t" data-backup={b.fileName}>
                  <td className="px-3 py-2 tabular-nums">
                    {formatDate(b.createdAt.slice(0, 10))} {b.createdAt.slice(11, 16)}
                  </td>
                  <td className="px-3 py-2">
                    <Badge variant={b.reason === "manual" ? "secondary" : "outline"}>
                      {REASON_LABEL[b.reason]}
                    </Badge>
                  </td>
                  <td className="px-3 py-2 text-right tabular-nums">{formatSize(b.size)}</td>
                  <td className="px-3 py-1.5 text-right">
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={busy}
                      data-action="restore"
                      onClick={() =>
                        prepare.mutate({
                          path: b.path,
                          label: `${formatDate(b.createdAt.slice(0, 10))} ${b.createdAt.slice(11, 16)}`,
                        })
                      }
                    >
                      Pulihkan
                    </Button>
                    <Button
                      size="icon-sm"
                      variant="ghost"
                      title="Hapus backup"
                      disabled={busy || remove.isPending}
                      onClick={() => remove.mutate(b)}
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <AlertDialog open={pending !== null} onOpenChange={(o) => !o && setPending(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Pulihkan data dari backup?</AlertDialogTitle>
            <AlertDialogDescription>
              Seluruh data saat ini akan diganti dengan isi backup
              {pending && <span className="font-mono"> ({pending.label})</span>}:{" "}
              {pending?.check.companyName ?? "(tanpa profil perusahaan)"}, {pending?.check.journals}{" "}
              jurnal, {pending?.check.accounts} akun. Salinan pengaman data saat ini dibuat otomatis
              sehingga restore ini bisa dibatalkan.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Batal</AlertDialogCancel>
            <Button
              variant="destructive"
              data-confirm-restore
              disabled={restore.isPending}
              onClick={() => pending && restore.mutate(pending.path)}
            >
              {restore.isPending ? "Memulihkan..." : "Pulihkan"}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  );
}
