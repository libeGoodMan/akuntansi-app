-- Penanda akun Kas & Bank (dipakai kartu "Kas & Bank" dan grafik saldo kas di Dashboard).
-- Dipilih pengguna per akun, bukan ditebak dari kode, karena COA bisa diubah bebas.
ALTER TABLE accounts ADD COLUMN is_cash INTEGER NOT NULL DEFAULT 0;
