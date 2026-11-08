import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

const GROUPS: { title: string; rows: [string, string][] }[] = [
  {
    title: "Di mana saja",
    rows: [
      ["Ctrl + K", "Buka palet perintah (cari halaman, akun, jurnal)"],
      [
        "Alt + 1 … 8",
        "Pindah ke Dashboard, Akun, Jurnal, Buku Besar, Laporan, Saldo Awal, Periode, Pengaturan",
      ],
      ["Alt + N", "Jurnal baru"],
      ["?", "Tampilkan bantuan pintasan ini"],
    ],
  },
  {
    title: "Form jurnal",
    rows: [
      ["Tab", "Pindah antar kolom"],
      ["Enter", "Di baris terakhir: tambah baris; di baris lain: ke akun berikutnya"],
      ["Ctrl + S", "Simpan (draft, atau perubahan pada jurnal yang sudah diposting)"],
      ["Ctrl + Enter", "Posting (bila sudah seimbang)"],
      ["↑ ↓ lalu Enter", "Memilih akun pada daftar autocomplete"],
    ],
  },
  {
    title: "Dialog",
    rows: [["Esc", "Tutup dialog atau palet"]],
  },
];

/** Daftar pintasan keyboard. */
export function ShortcutsDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg" data-shortcuts>
        <DialogHeader>
          <DialogTitle>Pintasan keyboard</DialogTitle>
          <DialogDescription>Semua fitur tetap bisa dipakai dengan mouse.</DialogDescription>
        </DialogHeader>
        <div className="max-h-96 space-y-4 overflow-auto">
          {GROUPS.map((g) => (
            <section key={g.title}>
              <h3 className="mb-1.5 text-xs font-medium text-muted-foreground">{g.title}</h3>
              <dl className="space-y-1.5 text-sm">
                {g.rows.map(([k, d]) => (
                  <div key={k} className="flex gap-3">
                    <dt className="w-32 shrink-0">
                      <kbd className="rounded border bg-muted px-1.5 py-0.5 font-mono text-xs">
                        {k}
                      </kbd>
                    </dt>
                    <dd className="text-muted-foreground">{d}</dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
