import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

/** Tunda pengecekan otomatis sesaat setelah aplikasi terbuka supaya tidak memperlambat startup. */
const STARTUP_DELAY_MS = 5000;

/**
 * Hasil pengecekan pembaruan, dibagikan lewat cache. Pengecekan otomatis hanya sekali per sesi dan
 * diam-diam gagal (tanpa internet bukanlah masalah). `refetch` dipakai tombol "Periksa pembaruan".
 */
export function useUpdateCheck() {
  const [ready, setReady] = useState(false);
  useEffect(() => {
    const t = setTimeout(() => setReady(true), STARTUP_DELAY_MS);
    return () => clearTimeout(t);
  }, []);

  return useQuery({
    queryKey: ["update-check"],
    queryFn: api.checkUpdate,
    enabled: ready,
    staleTime: Infinity,
    gcTime: Infinity,
    retry: false,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
  });
}
