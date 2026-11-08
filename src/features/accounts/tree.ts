import type { Account } from "@/lib/api";

export interface Row {
  account: Account;
  depth: number;
  hasChildren: boolean;
}

/**
 * Susun daftar akun menjadi baris berurutan pohon (induk lalu anak-anaknya, urut kode).
 * `filter` menyaring akun; induk dari akun yang cocok tetap ditampilkan agar konteksnya jelas.
 */
export function buildRows(accounts: Account[], filter: (a: Account) => boolean): Row[] {
  const byId = new Map(accounts.map((a) => [a.id, a]));
  const children = new Map<number | null, Account[]>();
  for (const a of accounts) {
    // Induk yang tidak ada (data rusak) dianggap akar supaya akun tidak hilang dari layar.
    const key = a.parentId !== null && byId.has(a.parentId) ? a.parentId : null;
    const list = children.get(key) ?? [];
    list.push(a);
    children.set(key, list);
  }
  for (const list of children.values()) list.sort((x, y) => x.code.localeCompare(y.code));

  const visible = new Set<number>();
  for (const a of accounts) {
    if (!filter(a)) continue;
    let cur: Account | undefined = a;
    while (cur && !visible.has(cur.id)) {
      visible.add(cur.id);
      cur = cur.parentId !== null ? byId.get(cur.parentId) : undefined;
    }
  }

  const rows: Row[] = [];
  const walk = (parent: number | null, depth: number) => {
    for (const a of children.get(parent) ?? []) {
      if (!visible.has(a.id)) continue;
      rows.push({ account: a, depth, hasChildren: (children.get(a.id) ?? []).length > 0 });
      walk(a.id, depth + 1);
    }
  };
  walk(null, 0);
  return rows;
}
