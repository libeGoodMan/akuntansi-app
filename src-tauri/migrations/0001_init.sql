-- Skema awal aplikasi akuntansi (double-entry). Semua nilai uang dalam satuan sen (INTEGER).

-- Profil perusahaan (satu baris)
CREATE TABLE company (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  name          TEXT NOT NULL,
  currency      TEXT NOT NULL DEFAULT 'IDR',
  fiscal_year_start_month INTEGER NOT NULL DEFAULT 1
);

-- Chart of Accounts
CREATE TABLE accounts (
  id            INTEGER PRIMARY KEY,
  code          TEXT NOT NULL UNIQUE,
  name          TEXT NOT NULL,
  type          TEXT NOT NULL CHECK (type IN ('ASSET','LIABILITY','EQUITY','REVENUE','EXPENSE')),
  normal_balance TEXT NOT NULL CHECK (normal_balance IN ('DEBIT','CREDIT')),
  parent_id     INTEGER REFERENCES accounts(id),
  is_header     INTEGER NOT NULL DEFAULT 0,
  is_active     INTEGER NOT NULL DEFAULT 1,
  is_system     INTEGER NOT NULL DEFAULT 0,    -- akun sistem (Modal Awal, Laba Ditahan) tidak bisa dihapus
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Periode akuntansi (untuk tutup buku)
CREATE TABLE fiscal_periods (
  id         INTEGER PRIMARY KEY,
  year       INTEGER NOT NULL,
  month      INTEGER NOT NULL CHECK (month BETWEEN 1 AND 12),
  is_closed  INTEGER NOT NULL DEFAULT 0,
  UNIQUE (year, month)
);

-- Penghitung nomor jurnal per bulan (nomor yang dihapus tidak dipakai ulang)
CREATE TABLE journal_counters (
  year      INTEGER NOT NULL,
  month     INTEGER NOT NULL CHECK (month BETWEEN 1 AND 12),
  last_seq  INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (year, month)
);

-- Header jurnal
CREATE TABLE journal_entries (
  id           INTEGER PRIMARY KEY,
  number       TEXT NOT NULL UNIQUE,           -- JU-2026-11-0001
  date         TEXT NOT NULL,                  -- YYYY-MM-DD
  description  TEXT NOT NULL,
  reference    TEXT,
  status       TEXT NOT NULL CHECK (status IN ('DRAFT','POSTED')) DEFAULT 'DRAFT',
  type         TEXT NOT NULL CHECK (type IN ('GENERAL','OPENING','CLOSING')) DEFAULT 'GENERAL',
  created_at   TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at   TEXT,
  posted_at    TEXT
);

-- Baris jurnal
CREATE TABLE journal_lines (
  id         INTEGER PRIMARY KEY,
  entry_id   INTEGER NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
  account_id INTEGER NOT NULL REFERENCES accounts(id),
  debit      INTEGER NOT NULL DEFAULT 0 CHECK (debit >= 0),
  credit     INTEGER NOT NULL DEFAULT 0 CHECK (credit >= 0),
  memo       TEXT,
  CHECK ((debit = 0) <> (credit = 0))
);

CREATE INDEX idx_lines_account ON journal_lines(account_id);
CREATE INDEX idx_lines_entry   ON journal_lines(entry_id);
CREATE INDEX idx_entries_date  ON journal_entries(date, status);

-- Hanya boleh ada satu jurnal saldo awal
CREATE UNIQUE INDEX idx_entries_one_opening ON journal_entries(type) WHERE type = 'OPENING';

-- Jejak audit
CREATE TABLE audit_log (
  id         INTEGER PRIMARY KEY,
  at         TEXT NOT NULL DEFAULT (datetime('now')),
  action     TEXT NOT NULL,
  entity     TEXT NOT NULL,
  entity_id  INTEGER,
  detail     TEXT
);
