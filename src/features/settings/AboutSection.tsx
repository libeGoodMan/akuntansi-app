import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Link } from "react-router-dom";
import { api, errorMessage } from "@/lib/api";
import { UpdateDialog } from "@/features/update/UpdateDialog";

type Status =
  { kind: "idle" } | { kind: "checking" } | { kind: "latest" } | { kind: "error"; message: string };

/** Versi aplikasi dan tombol periksa pembaruan manual. */
export function AboutSection() {
  const queryClient = useQueryClient();
  const info = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [open, setOpen] = useState(false);
  const found = useQuery({
    queryKey: ["update-check"],
    queryFn: api.checkUpdate,
    enabled: false,
    staleTime: Infinity,
  });

  async function check() {
    setStatus({ kind: "checking" });
    try {
      const update = await api.checkUpdate();
      queryClient.setQueryData(["update-check"], update);
      setStatus(update ? { kind: "idle" } : { kind: "latest" });
    } catch (e) {
      setStatus({ kind: "error", message: errorMessage(e) });
    }
  }

  const update = found.data ?? null;

  return (
    <section className="space-y-3 rounded-xl border p-5" data-about-section>
      <div>
        <h2 className="text-sm font-medium">Tentang dan pembaruan</h2>
        <p className="text-xs text-muted-foreground">
          Pembaruan tidak pernah dipasang otomatis; Anda memilih kapan.
        </p>
      </div>
      <dl className="grid grid-cols-[9rem_1fr] gap-y-1 text-sm">
        <dt className="text-muted-foreground">Versi aplikasi</dt>
        <dd data-app-version>{info.data?.version ?? "..."}</dd>
        <dt className="text-muted-foreground">Versi skema data</dt>
        <dd>{info.data?.schemaVersion ?? "..."}</dd>
        <dt className="text-muted-foreground">Lokasi data</dt>
        <dd className="font-mono text-xs break-all">{info.data?.dbPath ?? "..."}</dd>
      </dl>
      <div className="flex flex-wrap items-center gap-3">
        <Button
          size="sm"
          variant="outline"
          disabled={status.kind === "checking"}
          onClick={check}
          data-action="check-update"
        >
          {status.kind === "checking" ? "Memeriksa..." : "Periksa pembaruan"}
        </Button>
        {status.kind === "latest" && (
          <span className="text-sm text-muted-foreground" data-update-status="latest">
            Anda sudah memakai versi terbaru.
          </span>
        )}
        {status.kind === "error" && (
          <span className="text-sm text-destructive" data-update-status="error">
            {status.message}
          </span>
        )}
        {update && (
          <span className="flex items-center gap-2 text-sm" data-update-status="available">
            Versi {update.version} tersedia.
            <Button size="sm" onClick={() => setOpen(true)}>
              Lihat
            </Button>
          </span>
        )}
      </div>
      <p className="text-sm">
        <Link to="/lisensi" className="underline underline-offset-2" data-link-licenses>
          Lisensi pihak ketiga
        </Link>
      </p>
      {open && update && <UpdateDialog update={update} onClose={() => setOpen(false)} />}
    </section>
  );
}
