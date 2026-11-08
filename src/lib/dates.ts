const MONTHS_SHORT = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "Mei",
  "Jun",
  "Jul",
  "Agu",
  "Sep",
  "Okt",
  "Nov",
  "Des",
];

/** Tanggal lokal hari ini sebagai YYYY-MM-DD (bukan UTC). */
export function todayISO(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** "2026-11-05" -> "05 Nov 2026" */
export function formatDate(iso: string): string {
  const [y, m, d] = iso.split("-");
  const month = MONTHS_SHORT[Number(m) - 1];
  return month ? `${d} ${month} ${y}` : iso;
}

/** Waktu dari SQLite ("2026-11-05 14:03:00", UTC) ke waktu lokal yang mudah dibaca. */
export function formatDateTime(sqlite: string): string {
  const date = new Date(sqlite.replace(" ", "T") + "Z");
  if (Number.isNaN(date.getTime())) return sqlite;
  return date.toLocaleString("id-ID", { dateStyle: "medium", timeStyle: "short" });
}

const MONTHS_LONG = [
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

/** "2026-11" -> "Nov 26" (sumbu grafik) */
export function monthShort(ym: string): string {
  const [y, m] = ym.split("-");
  const name = MONTHS_SHORT[Number(m) - 1];
  return name ? `${name} ${y.slice(2)}` : ym;
}

/** "2026-11" -> "November 2026" */
export function monthLong(ym: string): string {
  const [y, m] = ym.split("-");
  const name = MONTHS_LONG[Number(m) - 1];
  return name ? `${name} ${y}` : ym;
}
