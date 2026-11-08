import {
  BookOpen,
  CalendarCheck,
  FileBarChart,
  LayoutDashboard,
  ListTree,
  NotebookPen,
  Scale,
  Settings,
} from "lucide-react";

/** Menu utama. Urutannya juga menentukan pintasan Alt+1 ... Alt+8. */
export const NAV_ITEMS = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard },
  { to: "/akun", label: "Chart of Accounts", icon: ListTree },
  { to: "/jurnal", label: "Jurnal Umum", icon: NotebookPen },
  { to: "/buku-besar", label: "Buku Besar", icon: BookOpen },
  { to: "/laporan", label: "Laporan", icon: FileBarChart },
  { to: "/saldo-awal", label: "Saldo Awal", icon: Scale },
  { to: "/periode", label: "Periode", icon: CalendarCheck },
  { to: "/pengaturan", label: "Pengaturan", icon: Settings },
] as const;
