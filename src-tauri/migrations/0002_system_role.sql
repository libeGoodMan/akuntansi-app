-- Peran akun sistem (dicari aplikasi lewat peran, bukan lewat kode akun yang bisa diubah user).
--   OPENING_EQUITY    = Modal / Saldo Awal Ekuitas (lawan jurnal saldo awal)
--   RETAINED_EARNINGS = Laba Ditahan (tujuan jurnal penutup akhir tahun)
ALTER TABLE accounts ADD COLUMN system_role TEXT
  CHECK (system_role IN ('OPENING_EQUITY','RETAINED_EARNINGS'));

CREATE UNIQUE INDEX idx_accounts_system_role ON accounts(system_role) WHERE system_role IS NOT NULL;
