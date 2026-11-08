# Panduan rilis

Aplikasi dirilis sebagai installer **NSIS per-pengguna** (tanpa hak admin) dengan **auto-update**. Pembaruan
tidak pernah dipasang otomatis: aplikasi hanya memberi tahu, pengguna memilih kapan memasang, dan data
dibackup otomatis sebelum update dipasang.

## Yang harus Anda siapkan satu kali

### 1. Alamat pembaruan
`src-tauri/tauri.conf.json` → `plugins.updater.endpoints` sudah diarahkan ke GitHub Releases:
`https://github.com/libeGoodMan/akuntansi-app/releases/latest/download/latest.json`
Workflow `release.yml` mengunggah `latest.json` ke setiap rilis. **Repo harus publik** (repo privat tidak bisa
dipakai karena aplikasi tidak membawa token). Bila nama pemilik atau repo berbeda, ubah alamat itu sebelum rilis.

### 2. Kunci penandatangan updater
Kunci sudah dibuat **dengan kata sandi** di komputer ini:

| Berkas | Isi |
|---|---|
| `%USERPROFILE%\.tauri\akuntansi-updater-v2.key` | **Kunci privat. RAHASIA.** Terenkripsi kata sandi. |
| `%USERPROFILE%\.tauri\akuntansi-updater-v2.key.pub` | Kunci publik (sudah ada di `tauri.conf.json`). |
| `akuntansi-updater-LAMA-tanpa-sandi.key*` | Kunci percobaan lama tanpa kata sandi. **Tidak dipakai lagi; hapus.** |

- **Simpan kunci privat dan kata sandinya di password manager, plus satu salinan di tempat lain.** Bila salah satunya
  hilang, aplikasi yang sudah terpasang **tidak bisa menerima update lagi** (kunci publik tertanam di aplikasi).
- Jangan mengganti kunci setelah ada pengguna: penggantian memutus update mereka.
- Jangan pernah meng-commit `*.key` (sudah ada di `.gitignore`).

### 3. Secret di GitHub (Settings → Secrets and variables → Actions)
| Secret | Isi |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | isi berkas `akuntansi-updater-v2.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | kata sandi kunci (wajib; kunci ini terenkripsi) |
| `WINDOWS_CERTIFICATE` | berkas `.pfx` code signing dalam base64 (opsional) |
| `WINDOWS_CERTIFICATE_PASSWORD` | kata sandi `.pfx` (opsional) |

### 4. Sertifikat code signing (keputusan: beta tanpa tanda tangan)
Versi beta dirilis **tanpa** tanda tangan Windows, sehingga SmartScreen menampilkan peringatan
"Windows protected your PC" (pengguna klik *More info → Run anyway*). Ini dijelaskan di `PANDUAN.md`, dan
workflow mencantumkan SHA-256 installer di catatan rilis agar pengguna bisa memeriksa keaslian berkas.
Pembaruan di dalam aplikasi tidak terpengaruh karena memakai tanda tangan updater yang terpisah.

Pilihan ke depan (bisa diganti kapan saja tanpa mengubah aplikasi maupun kunci updater):
- **SignPath Foundation** (gratis untuk proyek open source): perlu repo publik dengan lisensi yang diakui OSI,
  build lewat GitHub Actions, dan penerbit yang tampil adalah "SignPath Foundation". **Catatan teknis:** tanda tangan
  updater dihitung dari bytes installer, jadi bila installer ditandatangani layanan eksternal, tanda tangan updater dan
  `latest.json` harus dibuat SETELAH itu (`tauri signer sign` pada installer yang sudah bertanda tangan).
- **Sertifikat OV/EV berbayar** atau layanan tanda tangan cloud: peringatan SmartScreen hilang setelah reputasi
  terbentuk (EV langsung). Reputasi mulai dari nol bila berganti penerbit.

Setelah memiliki sertifikat `.pfx`, isi secret `WINDOWS_CERTIFICATE` dan `WINDOWS_CERTIFICATE_PASSWORD`;
workflow otomatis menandatangani installer bila secret itu ada.

## Membuat rilis

1. Naikkan versi di ketiga tempatnya sekaligus:
   ```
   node scripts/version.mjs 0.2.0
   ```
   (`node scripts/version.mjs` tanpa argumen hanya memeriksa ketiganya sama; CI melakukannya.)
2. Commit, lalu buat tag dan dorong:
   ```
   git tag v0.2.0
   git push origin main v0.2.0
   ```
3. Workflow **Release** membangun installer, menandatanganinya, dan membuat **draft** rilis di GitHub
   berisi: `Akuntansi_0.2.0_x64-setup.exe`, paket updater (`.exe` + `.sig`), dan `latest.json`.
4. Periksa draft, tulis catatan rilis, lalu **Publish**. Begitu dipublikasikan, pengguna yang sudah memasang
   versi lama melihat pita "Versi 0.2.0 tersedia".

## Build lokal (tanpa CI)
```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\akuntansi-updater-v2.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "<kata sandi kunci>"
npm run tauri build
```
Hasil di `src-tauri\target\release\bundle\nsis\`. Build pertama mengunduh NSIS otomatis dan memakan waktu
lama (kompilasi rilis penuh dengan LTO).

## Yang terjadi pada data pengguna saat update
- Sebelum installer berjalan, aplikasi membuat backup jenis **pra-update** (disimpan 5 terbaru).
- Bila versi baru mengubah skema database, migrasi berjalan saat pertama dibuka dan didahului backup
  **pra-migrasi**. Bila backup itu gagal, migrasi dibatalkan.
- Bila ada masalah: Pengaturan → Backup dan restore → pilih backup pra-update → **Pulihkan**
  (atau pasang kembali installer versi sebelumnya lalu pulihkan backup).

## Keamanan updater
- Paket pembaruan diverifikasi dengan kunci publik di aplikasi sebelum dipasang; paket tanpa tanda tangan
  yang sah ditolak.
- `requireSignedVersion` aktif: versi yang diumumkan `latest.json` harus sama dengan versi yang ditandatangani,
  sehingga penyerang tidak bisa memaksa downgrade ke rilis lama yang sah.
- Hanya HTTPS yang diterima untuk alamat pembaruan.

## Lisensi pihak ketiga
- `npm run build` otomatis menjalankan `scripts/licenses.mjs`, yang membuat `public/third-party-licenses.json`
  (dibaca halaman *Pengaturan → Lisensi pihak ketiga*) dan `THIRD-PARTY-NOTICES.txt` (ikut dipasang bersama
  installer). Keduanya dibuat ulang setiap build dan tidak di-commit.
- Skrip memerlukan `cargo` dan `node_modules` (jalankan `npm ci` dulu). Daftar mencakup crate Rust dan paket npm
  produksi beserta turunannya; font PDF ter-embed (Libertinus Serif, New Computer Modern, DejaVu Sans Mono) tercakup
  lewat berkas NOTICE crate `typst-assets`.
- Lima paket berlisensi **MPL-2.0** (cssparser, cssparser-macros, dtoa-short, option-ext, selectors) dipakai tanpa
  modifikasi. Bila suatu saat Anda mengubah kodenya, kode sumber berkas yang diubah wajib dibuka.
- Setelah menambah dependency baru, periksa keluaran skrip: baris "paket tanpa berkas teks lisensi" menandai paket
  yang perlu dicek manual, dan `cargo deny check licenses` menolak lisensi yang tidak diizinkan.

## Windows 10 dan Windows 11
- Aplikasi memakai **WebView2** (bagian dari Microsoft Edge). Windows 11 membawanya; Windows 10 yang rutin diperbarui
  umumnya sudah punya. Installer memakai mode `downloadBootstrapper`: bila WebView2 belum ada, installer mengunduhnya
  (butuh internet **saat memasang**).
- Untuk komputer tanpa internet saat pemasangan, ubah `bundle.windows.webviewInstallMode` di `tauri.conf.json`:
  - `embedBootstrapper`: installer lebih besar sekitar 1,8 MB, tetap butuh internet untuk mengunduh runtime.
  - `offlineInstaller`: menyertakan runtime lengkap (installer +120 MB), tidak butuh internet sama sekali.
- Yang sudah diuji pada pengembangan ini hanya Windows 11. **Uji pemasangan di Windows 10 bersih sebelum rilis**.
