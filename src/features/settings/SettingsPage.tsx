import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api, errorMessage, type Company } from "@/lib/api";
import { AboutSection } from "./AboutSection";
import { BackupSection } from "./BackupSection";

/** Pengaturan. Backup, tutup periode, dan lainnya menyusul di Fase 6. */
export function SettingsPage() {
  const company = useQuery({ queryKey: ["company"], queryFn: api.getCompany });
  return (
    <div className="mx-auto max-w-2xl space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Pengaturan</h1>
        <p className="text-sm text-muted-foreground">
          Profil perusahaan, backup, dan restore data.
        </p>
      </div>
      {company.error && <p className="text-sm text-destructive">{errorMessage(company.error)}</p>}
      {company.data && (
        <CompanyForm key={`${company.data.name}|${company.data.address}`} company={company.data} />
      )}
      <BackupSection />
      <AboutSection />
    </div>
  );
}

function CompanyForm({ company }: { company: Company }) {
  const queryClient = useQueryClient();
  const [name, setName] = useState(company.name);
  const [address, setAddress] = useState(company.address ?? "");
  const [error, setError] = useState<string | null>(null);

  const save = useMutation({
    mutationFn: () => api.updateCompany({ name, address: address.trim() || null }),
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: ["company"] });
      toast.success("Profil perusahaan disimpan");
    },
    onError: (e) => setError(errorMessage(e)),
  });

  const dirty = name !== company.name || (address.trim() || null) !== company.address;

  return (
    <form
      className="space-y-4 rounded-xl border p-5"
      onSubmit={(e) => {
        e.preventDefault();
        save.mutate();
      }}
    >
      <div>
        <h2 className="text-sm font-medium">Profil perusahaan</h2>
        <p className="text-xs text-muted-foreground">Tampil di kop semua laporan PDF dan Excel.</p>
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="s-name">Nama perusahaan</Label>
        <Input id="s-name" value={name} maxLength={120} onChange={(e) => setName(e.target.value)} />
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="s-address">Alamat</Label>
        <Input
          id="s-address"
          value={address}
          maxLength={300}
          placeholder="Jl. Merdeka No. 1, Jakarta Pusat 10110"
          onChange={(e) => setAddress(e.target.value)}
        />
      </div>
      {error && <p className="text-sm text-destructive">{error}</p>}
      <div className="flex justify-end">
        <Button type="submit" disabled={save.isPending || !dirty || !name.trim()}>
          {save.isPending ? "Menyimpan..." : "Simpan"}
        </Button>
      </div>
    </form>
  );
}
