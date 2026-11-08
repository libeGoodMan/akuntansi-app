import { lazy } from "react";
import { HashRouter, Route, Routes } from "react-router-dom";
import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Layout } from "@/components/Layout";
import { Toaster } from "@/components/ui/sonner";
import { NotFound } from "@/pages/NotFound";
import { SetupWizard } from "@/features/setup/SetupWizard";
import { api, errorMessage } from "@/lib/api";
import { useTheme } from "@/lib/theme";
import { DashboardPage } from "@/features/dashboard/DashboardPage";

/** Hanya di build dev: memaksa galat render untuk menguji ErrorBoundary. Tidak ikut build rilis. */
function Crash(): never {
  throw new Error("Galat uji yang disengaja");
}

// Halaman selain Dashboard dimuat saat pertama dibuka, supaya JavaScript awal kecil dan aplikasi cepat terbuka.
const AccountsPage = lazy(() =>
  import("@/features/accounts/AccountsPage").then((m) => ({ default: m.AccountsPage })),
);
const JournalFormPage = lazy(() =>
  import("@/features/journal/JournalFormPage").then((m) => ({ default: m.JournalFormPage })),
);
const JournalListPage = lazy(() =>
  import("@/features/journal/JournalListPage").then((m) => ({ default: m.JournalListPage })),
);
const LedgerPage = lazy(() =>
  import("@/features/ledger/LedgerPage").then((m) => ({ default: m.LedgerPage })),
);
const OpeningPage = lazy(() =>
  import("@/features/opening/OpeningPage").then((m) => ({ default: m.OpeningPage })),
);
const PeriodsPage = lazy(() =>
  import("@/features/periods/PeriodsPage").then((m) => ({ default: m.PeriodsPage })),
);
const ReportsPage = lazy(() =>
  import("@/features/reports/ReportsPage").then((m) => ({ default: m.ReportsPage })),
);
const LicensesPage = lazy(() =>
  import("@/features/settings/LicensesPage").then((m) => ({ default: m.LicensesPage })),
);
const SettingsPage = lazy(() =>
  import("@/features/settings/SettingsPage").then((m) => ({ default: m.SettingsPage })),
);

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } },
});

/** Tampilkan wizard setup sampai profil perusahaan terisi. */
function SetupGate({ children }: { children: React.ReactNode }) {
  const { data, error, isLoading } = useQuery({ queryKey: ["company"], queryFn: api.getCompany });

  if (isLoading) return null;
  if (error) {
    return <p className="p-8 text-sm text-destructive">{errorMessage(error)}</p>;
  }
  if (!data) return <SetupWizard />;
  return <>{children}</>;
}

export default function App() {
  useTheme(); // terapkan tema tersimpan sejak awal, termasuk di wizard
  return (
    <QueryClientProvider client={queryClient}>
      <ErrorBoundary>
        <SetupGate>
          <HashRouter>
            <Routes>
              <Route element={<Layout />}>
                <Route index element={<DashboardPage />} />
                <Route path="akun" element={<AccountsPage />} />
                <Route path="jurnal" element={<JournalListPage />} />
                <Route path="jurnal/baru" element={<JournalFormPage />} />
                <Route path="jurnal/:id" element={<JournalFormPage />} />
                <Route path="buku-besar" element={<LedgerPage />} />
                <Route path="laporan" element={<ReportsPage />} />
                <Route path="saldo-awal" element={<OpeningPage />} />
                <Route path="periode" element={<PeriodsPage />} />
                <Route path="pengaturan" element={<SettingsPage />} />
                <Route path="lisensi" element={<LicensesPage />} />
                {import.meta.env.DEV && <Route path="__crash" element={<Crash />} />}
                <Route path="*" element={<NotFound />} />
              </Route>
            </Routes>
          </HashRouter>
        </SetupGate>
      </ErrorBoundary>
      <Toaster position="bottom-right" />
    </QueryClientProvider>
  );
}
