import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { FileDown, FileSpreadsheet, Printer } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api, errorMessage, type ExportFormat, type ReportRequest } from "@/lib/api";

interface Props {
  request: ReportRequest;
  /** Nama berkas yang disarankan, tanpa ekstensi. */
  name: string;
  /** Format yang ditawarkan (bukti jurnal hanya PDF). */
  formats?: ExportFormat[];
  disabled?: boolean;
}

const FORMAT_LABEL: Record<ExportFormat, string> = { pdf: "PDF", xlsx: "Excel" };

/** Tombol Cetak, Simpan PDF, dan Simpan Excel untuk satu laporan. */
export function ExportButtons({ request, name, formats = ["pdf", "xlsx"], disabled }: Props) {
  const [busy, setBusy] = useState<string | null>(null);

  async function run(key: string, task: () => Promise<void>) {
    setBusy(key);
    try {
      await task();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  }

  async function saveAs(format: ExportFormat) {
    let path = await save({
      defaultPath: `${name}.${format}`,
      filters: [{ name: FORMAT_LABEL[format], extensions: [format] }],
    });
    if (!path) return; // dibatalkan
    if (!path.toLowerCase().endsWith(`.${format}`)) path += `.${format}`;
    await api.exportReport(request, format, path);
    toast.success(`${FORMAT_LABEL[format]} tersimpan: ${path}`);
  }

  const off = disabled || busy !== null;

  return (
    <div className="flex gap-2" data-export-buttons>
      {formats.includes("pdf") && (
        <Button
          variant="outline"
          size="sm"
          disabled={off}
          title="Buat PDF lalu buka di penampil PDF untuk dicetak"
          onClick={() => run("print", () => api.printReport(request))}
        >
          <Printer className="size-4" /> {busy === "print" ? "Membuat..." : "Cetak"}
        </Button>
      )}
      {formats.map((f) => (
        <Button
          key={f}
          variant="outline"
          size="sm"
          disabled={off}
          onClick={() => run(f, () => saveAs(f))}
        >
          {f === "pdf" ? <FileDown className="size-4" /> : <FileSpreadsheet className="size-4" />}
          {busy === f ? "Menyimpan..." : FORMAT_LABEL[f]}
        </Button>
      ))}
    </div>
  );
}
