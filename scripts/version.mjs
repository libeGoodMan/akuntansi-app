// Versi aplikasi ada di tiga tempat dan harus selalu sama (installer, updater, dan UI membacanya).
//
//   node scripts/version.mjs            cek ketiganya sama (dipakai CI; keluar dengan kode 1 bila beda)
//   node scripts/version.mjs 0.2.0      ubah ketiganya ke 0.2.0
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const files = {
  "package.json": {
    read: (t) => JSON.parse(t).version,
    write: (t, v) => t.replace(/("version":\s*")[^"]+(")/, `$1${v}$2`),
  },
  "src-tauri/tauri.conf.json": {
    read: (t) => JSON.parse(t).version,
    write: (t, v) => t.replace(/("version":\s*")[^"]+(")/, `$1${v}$2`),
  },
  "src-tauri/Cargo.toml": {
    read: (t) => t.match(/^version\s*=\s*"([^"]+)"/m)?.[1],
    write: (t, v) => t.replace(/^(version\s*=\s*")[^"]+(")/m, `$1${v}$2`),
  },
};

const read = (name) => fs.readFileSync(path.join(root, name), "utf8");
const arg = process.argv[2];

if (arg) {
  if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(arg)) {
    console.error(`Versi "${arg}" tidak valid. Contoh: 0.2.0 atau 1.0.0-beta.1`);
    process.exit(1);
  }
  for (const [name, f] of Object.entries(files)) {
    fs.writeFileSync(path.join(root, name), f.write(read(name), arg));
  }
  console.log(`Versi diubah ke ${arg}. Jalankan "cargo check" di src-tauri agar Cargo.lock ikut diperbarui.`);
} else {
  const versions = Object.fromEntries(Object.entries(files).map(([n, f]) => [n, f.read(read(n))]));
  const unique = new Set(Object.values(versions));
  for (const [n, v] of Object.entries(versions)) console.log(`${v ?? "(tidak ditemukan)"}  ${n}`);
  if (unique.size !== 1 || unique.has(undefined)) {
    console.error("Versi tidak sama di semua berkas.");
    process.exit(1);
  }
  console.log(`OK: semua ${[...unique][0]}`);
}
