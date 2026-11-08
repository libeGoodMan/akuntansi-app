/**
 * Uang selalu berupa bilangan bulat dalam sen (Rp 1.250.000,50 -> 125000050).
 * Jangan pernah menghitung uang dengan pecahan di JS; fungsi di sini hanya mengubah teks <-> sen.
 */

/** 125000050 -> "1.250.000,50" (tanda minus bila negatif). */
export function formatMoney(cents: number): string {
  const sign = cents < 0 ? "-" : "";
  const abs = Math.abs(Math.round(cents));
  const whole = Math.trunc(abs / 100)
    .toString()
    .replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  const frac = (abs % 100).toString().padStart(2, "0");
  return `${sign}${whole},${frac}`;
}

/** Seperti formatMoney, tetapi 0 ditampilkan sebagai string kosong (untuk sel debit/kredit). */
export function formatMoneyOrBlank(cents: number): string {
  return cents === 0 ? "" : formatMoney(cents);
}

export interface ParseOptions {
  allowNegative?: boolean;
}

/**
 * Teks -> sen. Format Indonesia: titik = pemisah ribuan, koma = desimal ("1.250.000,50").
 * Mengembalikan null bila kosong atau tidak valid (mis. lebih dari 2 desimal, huruf).
 */
export function parseMoney(text: string, opts: ParseOptions = {}): number | null {
  let s = text
    .trim()
    .replace(/^Rp\s*/i, "")
    .replace(/\s/g, "");
  if (s === "") return null;

  let negative = false;
  if (s.startsWith("-")) {
    if (!opts.allowNegative) return null;
    negative = true;
    s = s.slice(1);
  }

  const [intPart, fracPart, ...rest] = s.split(",");
  if (rest.length > 0) return null;
  if (!/^\d{1,3}(\.\d{3})*$|^\d+$/.test(intPart)) return null;
  if (fracPart !== undefined && !/^\d{1,2}$/.test(fracPart)) return null;

  const whole = Number(intPart.replace(/\./g, ""));
  const frac = Number((fracPart ?? "").padEnd(2, "0") || "0");
  const cents = whole * 100 + frac;
  if (!Number.isSafeInteger(cents)) return null;
  return negative ? -cents : cents;
}

/**
 * Angka ringkas untuk sumbu grafik: 1.250.000.000 -> "1,25 M", 12.500.000 -> "12,5 jt", 4.500 -> "4,5 rb".
 * Hanya untuk tampilan; nilai persisnya selalu lewat formatMoney.
 */
export function formatCompact(cents: number): string {
  const rupiah = cents / 100;
  const abs = Math.abs(rupiah);
  const fmt = (v: number, unit: string) =>
    `${v.toLocaleString("id-ID", { maximumFractionDigits: 2 })} ${unit}`;
  if (abs >= 1e12) return fmt(rupiah / 1e12, "T");
  if (abs >= 1e9) return fmt(rupiah / 1e9, "M");
  if (abs >= 1e6) return fmt(rupiah / 1e6, "jt");
  if (abs >= 1e3) return fmt(rupiah / 1e3, "rb");
  return rupiah.toLocaleString("id-ID", { maximumFractionDigits: 0 });
}
