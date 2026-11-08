import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { api, errorMessage, type SetupInput } from "@/lib/api";
import { cn } from "@/lib/utils";

const MONTHS = [
  "Januari",
  "Februari",
  "Maret",
  "April",
  "Mei",
  "Juni",
  "Juli",
  "Agustus",
  "September",
  "Oktober",
  "November",
  "Desember",
];

const TEMPLATES: { value: SetupInput["template"]; title: string; desc: string }[] = [
  {
    value: "standard",
    title: "COA standar Indonesia",
    desc: "Sekitar 50 akun siap pakai untuk usaha dagang/jasa: kas, bank, piutang, persediaan, aset tetap, utang, ekuitas, pendapatan, HPP, dan beban. Bisa diubah kapan saja.",
  },
  {
    value: "empty",
    title: "Mulai dari kosong",
    desc: "Hanya akun sistem (Modal Pemilik dan Laba Ditahan). Anda membuat sendiri semua akun lainnya.",
  },
];

export function SetupWizard() {
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [month, setMonth] = useState(1);
  const [template, setTemplate] = useState<SetupInput["template"]>("standard");

  const setup = useMutation({
    mutationFn: () => api.setupCompany({ name, fiscalYearStartMonth: month, template }),
    onSuccess: async () => {
      await queryClient.invalidateQueries();
      toast.success("Perusahaan siap. Selamat bekerja!");
    },
  });

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
      <form
        className="w-full max-w-xl space-y-6 rounded-2xl border bg-card p-8 text-card-foreground shadow-sm"
        onSubmit={(e) => {
          e.preventDefault();
          setup.mutate();
        }}
      >
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">Selamat datang</h1>
          <p className="text-sm text-muted-foreground">
            Isi profil perusahaan dan pilih daftar akun awal. Semua ini bisa diubah nanti.
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="company-name">Nama perusahaan</Label>
          <Input
            id="company-name"
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="PT Contoh Sejahtera"
            maxLength={120}
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="fy-start">Tahun fiskal dimulai bulan</Label>
          <NativeSelect
            id="fy-start"
            value={month}
            onChange={(e) => setMonth(Number(e.target.value))}
          >
            {MONTHS.map((m, i) => (
              <option key={m} value={i + 1}>
                {m}
              </option>
            ))}
          </NativeSelect>
        </div>

        <fieldset className="space-y-2">
          <legend className="mb-2 text-sm font-medium">Daftar akun (Chart of Accounts)</legend>
          {TEMPLATES.map((t) => (
            <label
              key={t.value}
              className={cn(
                "flex cursor-pointer gap-3 rounded-xl border p-4 transition-colors",
                template === t.value ? "border-primary bg-muted" : "hover:bg-muted/50",
              )}
            >
              <input
                type="radio"
                name="template"
                className="mt-1"
                checked={template === t.value}
                onChange={() => setTemplate(t.value)}
              />
              <span>
                <span className="block text-sm font-medium">{t.title}</span>
                <span className="block text-sm text-muted-foreground">{t.desc}</span>
              </span>
            </label>
          ))}
        </fieldset>

        {setup.error && <p className="text-sm text-destructive">{errorMessage(setup.error)}</p>}

        <Button type="submit" className="w-full" disabled={setup.isPending || !name.trim()}>
          {setup.isPending ? "Menyiapkan..." : "Mulai"}
        </Button>
      </form>
    </div>
  );
}
