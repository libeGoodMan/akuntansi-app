-- Tutup buku tahunan: satu baris per tahun fiskal yang sudah ditutup.
-- `year` = tahun kalender saat tahun fiskal itu DIMULAI (untuk tahun fiskal Jan-Des, sama dengan tahunnya).
-- `entry_id` = jurnal penutup (type CLOSING); NULL bila tahun itu tidak punya saldo pendapatan/beban.
CREATE TABLE fiscal_years (
  year       INTEGER PRIMARY KEY,
  closed_at  TEXT NOT NULL DEFAULT (datetime('now')),
  entry_id   INTEGER REFERENCES journal_entries(id) ON DELETE SET NULL,
  net_income INTEGER NOT NULL DEFAULT 0      -- laba (+) / rugi (-) yang dipindahkan ke Laba Ditahan, dalam sen
);
