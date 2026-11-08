import { Fragment, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { ArrowLeft, ChevronDown, ChevronRight } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { NativeSelect } from "@/components/ui/native-select";

interface Pkg {
  kind: "rust" | "npm" | "asset";
  name: string;
  version: string;
  license: string;
  repository: string | null;
  texts: string[];
}

interface LicenseData {
  generatedAt: string;
  packages: Pkg[];
  texts: Record<string, string>;
}

const KIND_LABEL = { rust: "Rust", npm: "npm", asset: "Aset" } as const;

/** Daftar lisensi dependency yang ikut terkirim. Datanya dibuat oleh scripts/licenses.mjs saat build. */
export function LicensesPage() {
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState("");
  const [open, setOpen] = useState<string | null>(null);

  const { data, error, isLoading } = useQuery({
    queryKey: ["third-party-licenses"],
    queryFn: async (): Promise<LicenseData> => {
      const r = await fetch("/third-party-licenses.json");
      if (!r.ok) throw new Error("Daftar lisensi tidak ditemukan pada build ini.");
      return r.json();
    },
    staleTime: Infinity,
  });

  const shown = useMemo(() => {
    const q = search.trim().toLowerCase();
    return (data?.packages ?? []).filter(
      (p) =>
        (kind === "" || p.kind === kind) &&
        (q === "" || p.name.toLowerCase().includes(q) || p.license.toLowerCase().includes(q)),
    );
  }, [data, search, kind]);

  const copyleft = data?.packages.filter((p) => /MPL|GPL|LGPL|EPL/i.test(p.license)) ?? [];

  return (
    <div className="mx-auto max-w-5xl space-y-5">
      <Link
        to="/pengaturan"
        className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft className="size-4" /> Pengaturan
      </Link>
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Lisensi pihak ketiga</h1>
        <p className="text-sm text-muted-foreground">
          Akuntansi dibangun dengan perangkat lunak sumber terbuka berikut. Font PDF (Libertinus
          Serif, New Computer Modern, DejaVu Sans Mono) dan font antarmuka Geist berlisensi SIL Open
          Font License atau setara. Berkas lengkapnya (THIRD-PARTY-NOTICES.txt) juga dipasang
          bersama aplikasi.
        </p>
      </div>

      {error && <p className="text-sm text-destructive">{(error as Error).message}</p>}
      {isLoading && <p className="text-sm text-muted-foreground">Memuat...</p>}

      {data && (
        <>
          {copyleft.length > 0 && (
            <p
              className="rounded-lg border bg-muted/30 px-3 py-2 text-xs text-muted-foreground"
              data-copyleft-note
            >
              {copyleft.length} paket berlisensi MPL-2.0 ({copyleft.map((p) => p.name).join(", ")})
              dipakai tanpa modifikasi; kode sumbernya tersedia di alamat repositori masing-masing.
            </p>
          )}
          <div className="flex flex-wrap items-center gap-3">
            <Input
              className="max-w-xs"
              placeholder="Cari nama paket atau lisensi"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
            <NativeSelect className="w-36" value={kind} onChange={(e) => setKind(e.target.value)}>
              <option value="">Semua jenis</option>
              <option value="rust">Rust</option>
              <option value="npm">npm</option>
              <option value="asset">Aset</option>
            </NativeSelect>
            <span className="text-sm text-muted-foreground" data-license-count>
              {shown.length} dari {data.packages.length} paket
            </span>
          </div>

          <div className="rounded-xl border">
            <table className="w-full text-sm">
              <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
                <tr>
                  <th className="w-8 rounded-tl-xl px-3 py-2.5" />
                  <th className="px-3 py-2.5 font-medium">Paket</th>
                  <th className="w-28 px-3 py-2.5 font-medium">Versi</th>
                  <th className="w-24 px-3 py-2.5 font-medium">Jenis</th>
                  <th className="rounded-tr-xl px-3 py-2.5 font-medium">Lisensi</th>
                </tr>
              </thead>
              <tbody>
                {shown.map((p) => {
                  const key = `${p.kind}:${p.name}@${p.version}`;
                  const isOpen = open === key;
                  return (
                    <Fragment key={key}>
                      <tr
                        className="cursor-pointer border-t hover:bg-muted/40"
                        data-package={p.name}
                        onClick={() => setOpen(isOpen ? null : key)}
                      >
                        <td className="px-3 py-2 text-muted-foreground">
                          {isOpen ? (
                            <ChevronDown className="size-4" />
                          ) : (
                            <ChevronRight className="size-4" />
                          )}
                        </td>
                        <td className="px-3 py-2 font-medium">{p.name}</td>
                        <td className="px-3 py-2 font-mono text-xs">{p.version}</td>
                        <td className="px-3 py-2">
                          <Badge variant="outline">{KIND_LABEL[p.kind]}</Badge>
                        </td>
                        <td className="px-3 py-2 text-muted-foreground">{p.license}</td>
                      </tr>
                      {isOpen && (
                        <tr className="border-t bg-muted/20">
                          <td colSpan={5} className="space-y-3 px-4 py-3">
                            {p.repository && (
                              <p className="text-xs break-all text-muted-foreground">
                                {p.repository}
                              </p>
                            )}
                            {p.texts.length === 0 && (
                              <p className="text-xs text-muted-foreground">
                                Paket ini tidak membawa berkas lisensi; lisensinya {p.license}.
                                Lihat repositori di atas.
                              </p>
                            )}
                            {p.texts.map((h) => (
                              <pre
                                key={h}
                                className="max-h-64 overflow-auto rounded-lg border bg-background p-3 text-xs whitespace-pre-wrap"
                              >
                                {data.texts[h]}
                              </pre>
                            ))}
                          </td>
                        </tr>
                      )}
                    </Fragment>
                  );
                })}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
