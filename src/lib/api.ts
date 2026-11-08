import { invoke } from "@tauri-apps/api/core";

/** Bentuk error dari Rust (lihat src-tauri/src/error.rs). */
export interface AppError {
  code: "DATABASE" | "MIGRATION" | "IO" | "VALIDATION" | "OTHER";
  message: string;
}

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}

export interface AppInfo {
  version: string;
  dbPath: string;
  schemaVersion: number;
  companyConfigured: boolean;
}

export type AccountType = "ASSET" | "LIABILITY" | "EQUITY" | "REVENUE" | "EXPENSE";
export type NormalBalance = "DEBIT" | "CREDIT";

export interface Account {
  id: number;
  code: string;
  name: string;
  type: AccountType;
  normalBalance: NormalBalance;
  parentId: number | null;
  isHeader: boolean;
  isActive: boolean;
  isSystem: boolean;
  systemRole: "OPENING_EQUITY" | "RETAINED_EARNINGS" | null;
  /** Akun kas/bank: dihitung di kartu Kas & Bank pada Dashboard. */
  isCash: boolean;
  /** Sudah dipakai di jurnal: tidak bisa dihapus, hanya dinonaktifkan. */
  inUse: boolean;
}

export interface AccountInput {
  code: string;
  name: string;
  type: AccountType;
  /** null = ikuti jenis akun; isi untuk akun kontra. */
  normalBalance: NormalBalance | null;
  parentId: number | null;
  isHeader: boolean;
  isCash: boolean;
}

export interface Company {
  name: string;
  currency: string;
  fiscalYearStartMonth: number;
  /** Alamat untuk kop laporan. */
  address: string | null;
}

export interface SetupInput {
  name: string;
  fiscalYearStartMonth: number;
  template: "standard" | "empty";
}

export type JournalStatus = "DRAFT" | "POSTED";
export type JournalType = "GENERAL" | "OPENING" | "CLOSING";

/** Nilai uang selalu bilangan bulat dalam sen. */
export interface JournalLineInput {
  accountId: number;
  debit: number;
  credit: number;
  memo: string | null;
}

export interface JournalInput {
  /** YYYY-MM-DD */
  date: string;
  description: string;
  reference: string | null;
  lines: JournalLineInput[];
}

export interface JournalLine {
  id: number;
  accountId: number;
  accountCode: string;
  accountName: string;
  debit: number;
  credit: number;
  memo: string | null;
}

export interface JournalDetail {
  id: number;
  number: string;
  date: string;
  description: string;
  reference: string | null;
  status: JournalStatus;
  type: JournalType;
  createdAt: string;
  updatedAt: string | null;
  postedAt: string | null;
  lines: JournalLine[];
}

export interface JournalSummary {
  id: number;
  number: string;
  date: string;
  description: string;
  reference: string | null;
  status: JournalStatus;
  type: JournalType;
  total: number;
  lineCount: number;
}

export interface JournalFilter {
  from?: string;
  to?: string;
  status?: JournalStatus | "";
  type?: JournalType | "";
  search?: string;
  page: number;
  pageSize: number;
}

export interface JournalPage {
  items: JournalSummary[];
  total: number;
  page: number;
  pageSize: number;
}

export interface HistoryItem {
  id: number;
  at: string;
  action: string;
  /** Isi sebelum/sesudah perubahan (bentuknya bergantung pada aksi). */
  detail: {
    before?: JournalDetail;
    after?: JournalDetail;
    number?: string;
  } | null;
}

export interface OpeningInput {
  date: string;
  lines: JournalLineInput[];
}

export interface LedgerQuery {
  accountId: number;
  from?: string;
  to?: string;
  page: number;
  pageSize: number;
}

export interface LedgerAccount {
  id: number;
  code: string;
  name: string;
  isHeader: boolean;
  normalBalance: NormalBalance;
}

export interface LedgerRow {
  date: string;
  entryId: number;
  number: string;
  description: string;
  memo: string | null;
  accountCode: string;
  accountName: string;
  debit: number;
  credit: number;
  /** Saldo berjalan searah saldo normal akun; negatif = berlawanan arah. */
  balance: number;
}

export interface LedgerPage {
  account: LedgerAccount;
  from: string | null;
  to: string | null;
  openingBalance: number;
  totalDebit: number;
  totalCredit: number;
  closingBalance: number;
  rows: LedgerRow[];
  total: number;
  page: number;
  pageSize: number;
}

export interface TrialBalanceQuery {
  asOf?: string;
  includeZero: boolean;
}

export interface TrialBalanceRow {
  accountId: number;
  code: string;
  name: string;
  type: AccountType;
  isActive: boolean;
  totalDebit: number;
  totalCredit: number;
  balanceDebit: number;
  balanceCredit: number;
}

export interface TrialBalance {
  asOf: string | null;
  rows: TrialBalanceRow[];
  totalDebit: number;
  totalCredit: number;
  difference: number;
  balanced: boolean;
}

export type LineKind = "heading" | "account" | "subtotal" | "total" | "net";

export interface StatementLine {
  kind: LineKind;
  level: number;
  label: string;
  code: string | null;
  accountId: number | null;
  amount: number | null;
  compare: number | null;
}

export interface IncomeStatementQuery {
  from?: string;
  to?: string;
  compareFrom?: string;
  compareTo?: string;
}

export interface IncomeStatement {
  from: string | null;
  to: string | null;
  compareFrom: string | null;
  compareTo: string | null;
  hasCompare: boolean;
  lines: StatementLine[];
  totalRevenue: number;
  totalExpense: number;
  /** Laba (positif) atau rugi (negatif). */
  netIncome: number;
  compareNetIncome: number | null;
}

export interface BalanceSheet {
  asOf: string | null;
  lines: StatementLine[];
  totalAssets: number;
  totalLiabilities: number;
  totalEquity: number;
  currentEarnings: number;
  totalLiabilitiesEquity: number;
  difference: number;
  balanced: boolean;
}

/** Permintaan ekspor/cetak; bentuknya sama dengan ReportRequest di Rust. */
export type ReportRequest =
  | { kind: "trial-balance"; asOf?: string; includeZero?: boolean }
  | ({ kind: "income-statement" } & IncomeStatementQuery)
  | { kind: "balance-sheet"; asOf?: string }
  | { kind: "ledger"; accountId: number; from?: string; to?: string }
  | { kind: "voucher"; id: number };

export type ExportFormat = "pdf" | "xlsx";

export interface UpdateCompanyInput {
  name: string;
  address: string | null;
}

export interface CashAccount {
  id: number;
  code: string;
  name: string;
  balance: number;
}

export interface MonthPoint {
  /** "2026-11" */
  month: string;
  revenue: number;
  expense: number;
  net: number;
  /** Saldo kas & bank pada akhir bulan itu. */
  cash: number;
}

export interface ExpenseSlice {
  label: string;
  code: string | null;
  amount: number;
}

export interface Dashboard {
  today: string;
  month: string;
  hasData: boolean;
  cashBalance: number;
  cashAccounts: CashAccount[];
  revenue: number;
  expense: number;
  net: number;
  prevRevenue: number;
  prevExpense: number;
  prevNet: number;
  monthly: MonthPoint[];
  expenseBreakdown: ExpenseSlice[];
  recent: JournalSummary[];
  draftCount: number;
}

export interface MonthStatus {
  year: number;
  month: number;
  closed: boolean;
  posted: number;
  drafts: number;
}

export interface YearClosing {
  closedAt: string;
  entryId: number | null;
  number: string | null;
  /** Laba (+) atau rugi (-) yang dipindahkan ke Laba Ditahan. */
  netIncome: number;
}

export interface PeriodOverview {
  fiscalYear: number;
  start: string;
  end: string;
  months: MonthStatus[];
  closing: YearClosing | null;
  /** Alasan tutup buku belum bisa dilakukan; kosong bila siap. */
  blockers: string[];
  canCloseYear: boolean;
  previewNetIncome: number;
  previewAccounts: number;
}

export type BackupReason = "keluar" | "pra-migrasi" | "pra-restore" | "pra-update" | "manual";

export interface BackupInfo {
  fileName: string;
  path: string;
  size: number;
  /** Waktu lokal, "YYYY-MM-DD HH:MM:SS". */
  createdAt: string;
  reason: BackupReason;
}

export interface BackupOverview {
  dir: string;
  backups: BackupInfo[];
}

export interface BackupCheck {
  schemaVersion: number;
  companyName: string | null;
  accounts: number;
  journals: number;
}

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string | null;
  date: string | null;
}

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

/** Semua pemanggilan ke Rust lewat sini supaya bertipe dan konsisten. */
export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
  getDashboard: (today?: string) => invoke<Dashboard>("get_dashboard", { query: { today } }),

  getCompany: () => invoke<Company | null>("get_company"),
  setupCompany: (input: SetupInput) => invoke<Company>("setup_company", { input }),

  listAccounts: () => invoke<Account[]>("list_accounts"),
  createAccount: (input: AccountInput) => invoke<Account>("create_account", { input }),
  updateAccount: (id: number, input: AccountInput) =>
    invoke<Account>("update_account", { id, input }),
  setAccountActive: (id: number, active: boolean) =>
    invoke<Account>("set_account_active", { id, active }),
  deleteAccount: (id: number) => invoke<void>("delete_account", { id }),

  listJournals: (filter: JournalFilter) => invoke<JournalPage>("list_journals", { filter }),
  getJournal: (id: number) => invoke<JournalDetail>("get_journal", { id }),
  createJournal: (input: JournalInput, post: boolean) =>
    invoke<JournalDetail>("create_journal", { input, post }),
  updateJournal: (id: number, input: JournalInput, post: boolean) =>
    invoke<JournalDetail>("update_journal", { id, input, post }),
  postJournal: (id: number) => invoke<JournalDetail>("post_journal", { id }),
  deleteJournal: (id: number) => invoke<void>("delete_journal", { id }),
  journalHistory: (id: number) => invoke<HistoryItem[]>("journal_history", { id }),

  getOpening: () => invoke<JournalDetail | null>("get_opening"),
  saveOpening: (input: OpeningInput) => invoke<JournalDetail>("save_opening", { input }),
  deleteOpening: () => invoke<void>("delete_opening"),

  getLedger: (query: LedgerQuery) => invoke<LedgerPage>("get_ledger", { query }),
  getTrialBalance: (query: TrialBalanceQuery) =>
    invoke<TrialBalance>("get_trial_balance", { query }),
  getIncomeStatement: (query: IncomeStatementQuery) =>
    invoke<IncomeStatement>("get_income_statement", { query }),
  getBalanceSheet: (asOf?: string) =>
    invoke<BalanceSheet>("get_balance_sheet", { query: { asOf } }),

  exportReport: (request: ReportRequest, format: ExportFormat, path: string) =>
    invoke<string>("export_report", { request, format, path }),
  printReport: (request: ReportRequest) => invoke<void>("print_report", { request }),

  updateCompany: (input: UpdateCompanyInput) => invoke<Company>("update_company", { input }),

  periodOverview: (fiscalYear: number) =>
    invoke<PeriodOverview>("get_period_overview", { fiscalYear }),
  fiscalYearFor: (date: string) => invoke<number>("fiscal_year_for", { date }),
  closePeriod: (year: number, month: number) => invoke<void>("close_period", { year, month }),
  reopenPeriod: (year: number, month: number) => invoke<void>("reopen_period", { year, month }),
  closeYear: (fiscalYear: number) => invoke<PeriodOverview>("close_year", { fiscalYear }),
  reopenYear: (fiscalYear: number) => invoke<PeriodOverview>("reopen_year", { fiscalYear }),

  listBackups: () => invoke<BackupOverview>("list_backups"),
  createBackup: () => invoke<string>("create_backup"),
  backupToPath: (path: string) => invoke<string>("backup_to_path", { path }),
  checkBackup: (path: string) => invoke<BackupCheck>("check_backup", { path }),
  restoreBackup: (path: string) => invoke<BackupCheck>("restore_backup", { path }),
  deleteBackup: (fileName: string) => invoke<void>("delete_backup", { fileName }),
};

export const ACCOUNT_TYPE_LABEL: Record<AccountType, string> = {
  ASSET: "Aset",
  LIABILITY: "Kewajiban",
  EQUITY: "Ekuitas",
  REVENUE: "Pendapatan",
  EXPENSE: "Beban",
};

export function defaultNormalBalance(type: AccountType): NormalBalance {
  return type === "ASSET" || type === "EXPENSE" ? "DEBIT" : "CREDIT";
}
