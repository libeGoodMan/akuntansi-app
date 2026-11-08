import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { ChevronLeft, ChevronRight, Plus, Search } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { NativeSelect } from "@/components/ui/native-select";
import { api, errorMessage, type JournalFilter, type JournalSummary } from "@/lib/api";
import { formatDate } from "@/lib/dates";
import { formatMoney } from "@/lib/money";
import { cn } from "@/lib/utils";

const PAGE_SIZE = 25;

const TYPE_LABEL: Record<string, string> = { OPENING: "Saldo awal", CLOSING: "Penutup" };

export function JournalListPage() {
  const navigate = useNavigate();
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [status, setStatus] = useState<JournalFilter["status"]>("");
  const [type, setType] = useState<JournalFilter["type"]>("");
  const [searchText, setSearchText] = useState("");
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);

  // Tunda pencarian 300 ms supaya tidak query di setiap ketikan.
  useEffect(() => {
    const t = setTimeout(() => setSearch(searchText), 300);
    return () => clearTimeout(t);
  }, [searchText]);

  const filter: JournalFilter = { from, to, status, type, search, page, pageSize: PAGE_SIZE };
  const { data, error, isLoading, isFetching } = useQuery({
    queryKey: ["journals", filter],
    queryFn: () => api.listJournals(filter),
    placeholderData: (prev) => prev,
  });

  const totalPages = data ? Math.max(1, Math.ceil(data.total / data.pageSize)) : 1;

  // Ubah filter selalu kembali ke halaman 1.
  const withReset =
    <T,>(set: (v: T) => void) =>
    (v: T) => {
      set(v);
      setPage(1);
    };

  function open(j: JournalSummary) {
    navigate(j.type === "OPENING" ? "/saldo-awal" : `/jurnal/${j.id}`);
  }

  const hasFilter = from || to || status || type || search;

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Jurnal Umum</h1>
          <p className="text-sm text-muted-foreground">
            {data ? `${data.total} jurnal` : "Memuat..."}
            {hasFilter ? " sesuai filter" : ""}
          </p>
        </div>
        <Button onClick={() => navigate("/jurnal/baru")}>
          <Plus className="size-4" /> Jurnal baru
        </Button>
      </div>

      <div className="flex flex-wrap items-end gap-3">
        <div className="relative w-72">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="pl-8"
            placeholder="Cari nomor, keterangan, referensi"
            value={searchText}
            onChange={(e) => {
              setSearchText(e.target.value);
              setPage(1);
            }}
          />
        </div>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Dari
          <Input
            type="date"
            className="w-40"
            value={from}
            onChange={(e) => withReset(setFrom)(e.target.value)}
          />
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Sampai
          <Input
            type="date"
            className="w-40"
            value={to}
            onChange={(e) => withReset(setTo)(e.target.value)}
          />
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Status
          <NativeSelect
            className="w-36"
            value={status}
            onChange={(e) => withReset(setStatus)(e.target.value as JournalFilter["status"])}
          >
            <option value="">Semua</option>
            <option value="DRAFT">Draft</option>
            <option value="POSTED">Diposting</option>
          </NativeSelect>
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Jenis
          <NativeSelect
            className="w-36"
            value={type}
            onChange={(e) => withReset(setType)(e.target.value as JournalFilter["type"])}
          >
            <option value="">Semua</option>
            <option value="GENERAL">Umum</option>
            <option value="OPENING">Saldo awal</option>
            <option value="CLOSING">Penutup</option>
          </NativeSelect>
        </label>
        {hasFilter && (
          <Button
            variant="ghost"
            onClick={() => {
              setFrom("");
              setTo("");
              setStatus("");
              setType("");
              setSearchText("");
              setSearch("");
              setPage(1);
            }}
          >
            Reset filter
          </Button>
        )}
      </div>

      {error && <p className="text-sm text-destructive">{errorMessage(error)}</p>}

      <div
        className={cn(
          "rounded-xl border",
          isFetching && !isLoading && "opacity-70 transition-opacity",
        )}
      >
        <table className="w-full text-sm">
          <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
            <tr>
              <th className="w-44 rounded-tl-xl px-4 py-2.5 font-medium">Nomor</th>
              <th className="w-28 px-4 py-2.5 font-medium">Tanggal</th>
              <th className="px-4 py-2.5 font-medium">Keterangan</th>
              <th className="w-28 px-4 py-2.5 font-medium">Status</th>
              <th className="w-40 rounded-tr-xl px-4 py-2.5 text-right font-medium">Total</th>
            </tr>
          </thead>
          <tbody>
            {isLoading && (
              <tr>
                <td colSpan={5} className="px-4 py-10 text-center text-muted-foreground">
                  Memuat...
                </td>
              </tr>
            )}
            {data && data.items.length === 0 && (
              <tr>
                <td colSpan={5} className="px-4 py-12 text-center text-muted-foreground">
                  {hasFilter
                    ? "Tidak ada jurnal yang cocok dengan filter."
                    : "Belum ada jurnal. Klik “Jurnal baru” untuk mulai."}
                </td>
              </tr>
            )}
            {data?.items.map((j) => (
              <tr
                key={j.id}
                data-journal={j.number}
                tabIndex={0}
                className="cursor-pointer border-t hover:bg-muted/40 focus-visible:bg-muted/40 focus-visible:outline-none"
                onClick={() => open(j)}
                onKeyDown={(e) => e.key === "Enter" && open(j)}
              >
                <td className="px-4 py-2.5 font-mono text-xs">{j.number}</td>
                <td className="px-4 py-2.5">{formatDate(j.date)}</td>
                <td className="px-4 py-2.5">
                  <div className="flex items-center gap-2">
                    <span>{j.description}</span>
                    {TYPE_LABEL[j.type] && <Badge variant="outline">{TYPE_LABEL[j.type]}</Badge>}
                  </div>
                  {j.reference && (
                    <div className="text-xs text-muted-foreground">Ref: {j.reference}</div>
                  )}
                </td>
                <td className="px-4 py-2.5">
                  <Badge variant={j.status === "POSTED" ? "secondary" : "outline"}>
                    {j.status === "POSTED" ? "Diposting" : "Draft"}
                  </Badge>
                </td>
                <td className="px-4 py-2.5 text-right tabular-nums">{formatMoney(j.total)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {data && data.total > 0 && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>
            Halaman {data.page} dari {totalPages}
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={page <= 1}
              onClick={() => setPage((p) => p - 1)}
            >
              <ChevronLeft className="size-4" /> Sebelumnya
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={page >= totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Berikutnya <ChevronRight className="size-4" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
