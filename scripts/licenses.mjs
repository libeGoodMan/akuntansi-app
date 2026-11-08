// Mengumpulkan lisensi semua dependency yang ikut terkirim ke pengguna:
//   - Rust : crate yang masuk build (dependency "normal", target Windows), dari `cargo metadata`
//   - npm  : dependency produksi (bukan devDependencies) beserta turunannya, dari node_modules
//   - aset : paket yang isinya (CSS) ikut tergabung ke bundel meski hanya alat saat build
// Hasil:
//   public/third-party-licenses.json   dibaca halaman "Lisensi pihak ketiga" di aplikasi
//   THIRD-PARTY-NOTICES.txt            ikut dipasang bersama installer
// Jalankan: node scripts/licenses.mjs   (otomatis dipanggil oleh `npm run build`)
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const MAX_TEXT = 200 * 1024; // berkas lisensi yang lebih besar dari ini hampir pasti bukan teks lisensi
const LICENSE_FILE = /^(LICEN[CS]E|COPYING|NOTICE|UNLICENSE)([-_.].*)?$/i;

/** Paket yang hanya alat build tetapi CSS-nya tergabung ke bundel: dicatat tanpa turunannya. */
const BUNDLED_ASSETS = ["shadcn", "tw-animate-css", "tailwindcss"];

const texts = new Map(); // hash -> teks
const packages = [];

function addText(raw) {
  const text = raw.replace(/\r\n/g, "\n").trim();
  if (!text) return null;
  const hash = crypto.createHash("sha1").update(text).digest("hex").slice(0, 12);
  texts.set(hash, text);
  return hash;
}

function readLicenseTexts(dir, extraFile) {
  const hashes = [];
  let names = [];
  try {
    names = fs.readdirSync(dir).filter((n) => LICENSE_FILE.test(n) && fs.statSync(path.join(dir, n)).isFile());
  } catch {
    /* folder tidak ada */
  }
  if (extraFile && !names.includes(extraFile)) names.push(extraFile);
  for (const n of names.sort()) {
    const file = path.join(dir, n);
    try {
      if (fs.statSync(file).size > MAX_TEXT) continue;
      const h = addText(fs.readFileSync(file, "utf8"));
      if (h && !hashes.includes(h)) hashes.push(h);
    } catch {
      /* berkas tak terbaca */
    }
  }
  return hashes;
}

// ---------------- Rust ----------------
function collectRust() {
  const out = execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-msvc", "--manifest-path", path.join(root, "src-tauri", "Cargo.toml")],
    { encoding: "utf8", maxBuffer: 1 << 29 },
  );
  const meta = JSON.parse(out);
  const byId = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));

  const seen = new Set();
  const stack = [meta.resolve.root];
  while (stack.length) {
    for (const d of nodes.get(stack.pop()).deps) {
      if (!d.dep_kinds.some((k) => k.kind === null)) continue; // lewati build/dev
      if (!seen.has(d.pkg)) {
        seen.add(d.pkg);
        stack.push(d.pkg);
      }
    }
  }
  for (const id of seen) {
    const p = byId.get(id);
    const dir = path.dirname(p.manifest_path);
    packages.push({
      kind: "rust",
      name: p.name,
      version: p.version,
      license: p.license ?? (p.license_file ? `(lihat ${p.license_file})` : "TIDAK DIKETAHUI"),
      repository: p.repository ?? null,
      texts: readLicenseTexts(dir, p.license_file ? path.basename(p.license_file) : null),
    });
  }
}

// ---------------- npm ----------------
function findPackageDir(name, fromDir) {
  let dir = fromDir;
  for (;;) {
    const candidate = path.join(dir, "node_modules", name);
    if (fs.existsSync(path.join(candidate, "package.json"))) return candidate;
    const parent = path.dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

function npmLicense(pkg) {
  if (typeof pkg.license === "string") return pkg.license;
  if (pkg.license?.type) return pkg.license.type;
  if (Array.isArray(pkg.licenses)) return pkg.licenses.map((l) => l.type ?? l).join(" OR ");
  return "TIDAK DIKETAHUI";
}

function repoOf(pkg) {
  const r = typeof pkg.repository === "string" ? pkg.repository : pkg.repository?.url;
  return r ? r.replace(/^git\+/, "").replace(/\.git$/, "") : (pkg.homepage ?? null);
}

function collectNpm() {
  const rootPkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
  const seen = new Map(); // dir -> true
  const queue = Object.keys(rootPkg.dependencies ?? {}).map((n) => ({ name: n, from: root }));
  const add = (dir, withDeps) => {
    if (seen.has(dir)) return;
    seen.set(dir, true);
    const pkg = JSON.parse(fs.readFileSync(path.join(dir, "package.json"), "utf8"));
    packages.push({
      kind: BUNDLED_ASSETS.includes(pkg.name) && !withDeps ? "asset" : "npm",
      name: pkg.name,
      version: pkg.version,
      license: npmLicense(pkg),
      repository: repoOf(pkg),
      texts: readLicenseTexts(dir, null),
    });
    if (withDeps) for (const n of Object.keys(pkg.dependencies ?? {})) queue.push({ name: n, from: dir });
  };
  while (queue.length) {
    const { name, from } = queue.shift();
    const dir = findPackageDir(name, from);
    if (!dir) {
      console.warn(`  ! paket npm tidak ditemukan: ${name} (jalankan npm ci)`);
      continue;
    }
    add(dir, true);
  }
  for (const name of BUNDLED_ASSETS) {
    const dir = findPackageDir(name, root);
    if (dir) add(dir, false);
  }
}

collectRust();
collectNpm();

// Crate dalam workspace (mis. typst-*) tidak membawa berkas LICENSE di foldernya. Untuk lisensi tanpa baris
// hak cipta (Apache-2.0, MPL-2.0) teks bakunya sama untuk semua, jadi pinjam dari paket lain yang membawanya.
const STANDARD = {
  // Hanya teks baku murni (diawali judulnya), bukan berkas gabungan dari satu proyek tertentu.
  "Apache-2.0": (t) => /^Apache License\s+Version 2\.0, January 2004/.test(t) && t.length < 14000,
  "MPL-2.0": (t) => /^Mozilla Public License Version 2\.0/.test(t) && t.length < 20000,
};
const standardHash = {};
for (const [id, test] of Object.entries(STANDARD)) {
  standardHash[id] = [...texts].find(([, t]) => test(t) && t.length > 8000)?.[0];
}
for (const p of packages) {
  if (p.texts.length) continue;
  for (const id of Object.keys(STANDARD)) {
    // Cocokkan ID SPDX utuh ("Apache-2.0" tidak boleh cocok dengan "Apache-2.0 WITH LLVM-exception" salah arah).
    const tokens = p.license.split(/[\s()/]+/);
    if (tokens.includes(id) && standardHash[id]) p.texts.push(standardHash[id]);
  }
}

packages.sort((a, b) => a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name) || a.version.localeCompare(b.version));

const unknown = packages.filter((p) => p.license === "TIDAK DIKETAHUI");
const noText = packages.filter((p) => p.texts.length === 0);

// ---- public/third-party-licenses.json ----
fs.mkdirSync(path.join(root, "public"), { recursive: true });
const used = new Set(packages.flatMap((p) => p.texts));
const json = {
  generatedAt: new Date().toISOString(),
  packages,
  texts: Object.fromEntries([...texts].filter(([h]) => used.has(h))),
};
fs.writeFileSync(path.join(root, "public", "third-party-licenses.json"), JSON.stringify(json));

// ---- THIRD-PARTY-NOTICES.txt (teks yang sama dicetak sekali, lalu daftar paket yang memakainya) ----
const lines = [];
lines.push("AKUNTANSI - PEMBERITAHUAN LISENSI PIHAK KETIGA", "=".repeat(60), "");
lines.push("Aplikasi ini memuat perangkat lunak pihak ketiga berikut. Daftar ringkas ada di Bagian 1,");
lines.push("teks lisensi lengkapnya di Bagian 2.", "");
lines.push(`BAGIAN 1 - DAFTAR (${packages.length} paket)`, "-".repeat(60));
for (const p of packages) lines.push(`${p.name} ${p.version}  [${p.license}]  (${p.kind})${p.repository ? "  " + p.repository : ""}`);
lines.push("", "", "BAGIAN 2 - TEKS LISENSI", "-".repeat(60), "");
const users = new Map();
for (const p of packages) for (const h of p.texts) (users.get(h) ?? users.set(h, []).get(h)).push(`${p.name} ${p.version}`);
const order = [...users.entries()].sort((a, b) => b[1].length - a[1].length);
for (const [hash, who] of order) {
  const head = who.length > 12 ? `${who.slice(0, 12).join(", ")}, dan ${who.length - 12} paket lain` : who.join(", ");
  lines.push("=".repeat(60), `Berlaku untuk: ${head}`, "=".repeat(60), texts.get(hash), "", "");
}
fs.writeFileSync(path.join(root, "THIRD-PARTY-NOTICES.txt"), lines.join("\n"));

const byLicense = {};
for (const p of packages) byLicense[p.license] = (byLicense[p.license] ?? 0) + 1;
const top = Object.entries(byLicense).sort((a, b) => b[1] - a[1]).slice(0, 8);
console.log(`Lisensi: ${packages.length} paket (${packages.filter((p) => p.kind === "rust").length} Rust, ${packages.filter((p) => p.kind !== "rust").length} npm/aset), ${texts.size} teks unik.`);
console.log("  teratas:", top.map(([l, n]) => `${l} x${n}`).join("; "));
if (unknown.length) console.warn(`  ! lisensi tidak diketahui: ${unknown.map((p) => p.name).join(", ")}`);
if (noText.length) console.warn(`  ! ${noText.length} paket tanpa berkas teks lisensi (hanya ID SPDX): ${noText.slice(0, 15).map((p) => p.name).join(", ")}${noText.length > 15 ? ", ..." : ""}`);
