import { useState } from "react";
import { ArrowDownToLine, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { UpdateDialog } from "./UpdateDialog";
import { useUpdateCheck } from "./useUpdateCheck";

/** Pita di atas halaman bila ada versi baru. Tidak memaksa: "Nanti" menyembunyikannya sampai aplikasi dibuka lagi. */
export function UpdateBanner() {
  const { data } = useUpdateCheck();
  const [dismissed, setDismissed] = useState(false);
  const [open, setOpen] = useState(false);

  if (!data || dismissed) return null;

  return (
    <>
      <div
        data-update-banner
        className="flex items-center gap-3 border-b bg-primary/5 px-6 py-2 text-sm"
        role="status"
      >
        <ArrowDownToLine className="size-4 shrink-0" aria-hidden />
        <span className="flex-1">
          Versi <strong>{data.version}</strong> tersedia (Anda memakai {data.currentVersion}).
        </span>
        <Button size="sm" onClick={() => setOpen(true)}>
          Lihat
        </Button>
        <Button size="icon-sm" variant="ghost" title="Nanti" onClick={() => setDismissed(true)}>
          <X className="size-4" />
        </Button>
      </div>
      {open && <UpdateDialog update={data} onClose={() => setOpen(false)} />}
    </>
  );
}
