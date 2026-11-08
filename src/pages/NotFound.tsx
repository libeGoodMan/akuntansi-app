import { Link } from "react-router-dom";
import { Button } from "@/components/ui/button";

export function NotFound() {
  return (
    <div className="mx-auto max-w-md space-y-3 py-16 text-center" data-not-found>
      <h1 className="text-2xl font-semibold tracking-tight">Halaman tidak ditemukan</h1>
      <p className="text-sm text-muted-foreground">
        Alamat yang Anda tuju tidak ada di aplikasi ini.
      </p>
      <Button render={<Link to="/" />}>Ke Dashboard</Button>
    </div>
  );
}
