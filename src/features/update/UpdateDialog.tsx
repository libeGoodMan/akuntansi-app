import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useMutation } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { api, errorMessage, type UpdateInfo, type UpdateProgress } from "@/lib/api";
import { formatDate } from "@/lib/dates";

interface Props {
  update: UpdateInfo;
  onClose: () => void;
}

function percent(p: UpdateProgress | null): number | null {
  if (!p || !p.total) return null;
  return Math.min(100, Math.round((p.downloaded / p.total) * 100));
}

/** Detail pembaruan dan tombol pasang. Pengguna yang memilih kapan memasang; "Nanti" selalu tersedia. */
export function UpdateDialog({ update, onClose }: Props) {
  const [progress, setProgress] = useState<UpdateProgress | null>(null);

  useEffect(() => {
    const un = listen<UpdateProgress>("update-progress", (e) => setProgress(e.payload));
    return () => {
      void un.then((f) => f());
    };
  }, []);

  const install = useMutation({ mutationFn: api.installUpdate });
  const pct = percent(progress);
  const busy = install.isPending;

  return (
    <Dialog open onOpenChange={(o) => !o && !busy && onClose()}>
      <DialogContent className="sm:max-w-lg" data-update-dialog>
        <DialogHeader>
          <DialogTitle>Pembaruan tersedia: versi {update.version}</DialogTitle>
          <DialogDescription>
            Versi Anda saat ini {update.currentVersion}
            {update.date ? ` · dirilis ${formatDate(update.date.slice(0, 10))}` : ""}.
          </DialogDescription>
        </DialogHeader>

        {update.notes && (
          <div className="max-h-48 overflow-auto rounded-lg border bg-muted/30 p-3 text-sm whitespace-pre-wrap">
            {update.notes}
          </div>
        )}

        <p className="text-xs text-muted-foreground">
          Sebelum dipasang, data Anda dibackup otomatis. Installer akan menutup aplikasi lalu
          membukanya kembali; pekerjaan yang belum disimpan akan hilang, jadi simpan dulu.
        </p>

        {busy && (
          <div className="space-y-1" role="status">
            <div className="h-2 overflow-hidden rounded-full bg-muted">
              <div
                className="h-full bg-primary transition-all"
                style={{ width: `${pct ?? 5}%` }}
                data-update-progress={pct ?? "indeterminate"}
              />
            </div>
            <p className="text-xs text-muted-foreground">
              {pct === null ? "Menyiapkan..." : `Mengunduh ${pct}%`}
            </p>
          </div>
        )}

        {install.error && (
          <p
            role="alert"
            className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
          >
            {errorMessage(install.error)}
          </p>
        )}

        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={onClose}>
            Nanti
          </Button>
          <Button disabled={busy} onClick={() => install.mutate()} data-action="install-update">
            {busy ? "Memasang..." : "Pasang sekarang"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
