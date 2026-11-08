# Panduan pengguna Akuntansi

Aplikasi pencatatan akuntansi untuk perusahaan: daftar akun, jurnal debit/kredit, buku besar, laporan keuangan,
dan dashboard. Semua data tersimpan **di komputer Anda** (tanpa internet, tanpa langganan).

## Memulai
1. **Pengaturan awal.** Saat pertama dibuka, isi nama perusahaan, bulan awal tahun fiskal, dan pilih daftar akun
   (Chart of Accounts): *COA standar Indonesia* (sekitar 50 akun siap pakai) atau *mulai dari kosong*.
2. **Isi alamat perusahaan** di Pengaturan agar tampil di kop semua laporan PDF dan Excel.
3. **Saldo awal** (menu Saldo Awal): bila Anda pindah dari pembukuan lama, isi saldo akun neraca per tanggal mulai
   pembukuan. Selisihnya otomatis masuk ke akun Modal Pemilik.

## Mencatat transaksi (Jurnal Umum)
- **Jurnal baru** (`Alt+N`): isi tanggal, keterangan, lalu baris akun dengan debit atau kredit. Selisih ditampilkan
  langsung; tombol **Posting** aktif hanya bila debit = kredit.
- **Draft** boleh belum seimbang dan belum dihitung di laporan. **Diposting** artinya sudah masuk laporan.
- Jurnal yang sudah diposting **tetap bisa diubah atau dihapus** (dengan konfirmasi). Setiap perubahan tercatat di
  Riwayat perubahan di bagian bawah jurnal, lengkap dengan isi sebelum dan sesudahnya.
- Nomor jurnal berurutan per bulan dan **tidak dipakai ulang**, meski jurnalnya dihapus.
- Akun yang sudah dipakai di jurnal tidak bisa dihapus, hanya dinonaktifkan.
- **Cetak bukti jurnal** (PDF) dari halaman jurnal; ada kotak tanda tangan.

## Laporan
Menu **Laporan** memuat Neraca Saldo, Laba Rugi (dengan pembanding periode lain), dan Neraca. Setiap laporan
bisa **dicetak**, **disimpan sebagai PDF**, atau **diekspor ke Excel**. Klik nama akun pada laporan untuk melihat
buku besarnya. **Buku Besar** menampilkan mutasi satu akun dengan saldo berjalan.

## Dashboard
Ringkasan bulan berjalan: Kas & Bank, pendapatan, beban, laba bersih (dibanding bulan lalu), grafik 12 bulan, dan
komposisi beban. Tandai akun mana yang termasuk "Kas & Bank" di Chart of Accounts (kotak *Akun kas/bank*).

## Menutup periode
Menu **Periode**:
- **Tutup periode** (per bulan) mengunci jurnal bulan itu. Bisa dibuka kembali.
- **Tutup buku tahunan** (setelah 12 bulan ditutup) membuat satu jurnal penutup: pendapatan dan beban dinolkan,
  labanya pindah ke Laba Ditahan. Laporan Laba Rugi tahun itu tidak berubah. Bisa dibatalkan.

## Menjaga data tetap aman
- **Backup otomatis** dibuat saat aplikasi ditutup (bila ada perubahan) dan sebelum update.
- **Backup manual** dan **Pulihkan**: Pengaturan → Backup dan restore. Sebelum memulihkan, aplikasi membuat salinan
  pengaman dari data saat ini.
- Backup tersimpan **di komputer yang sama** dengan datanya. Simpan juga satu salinan di tempat lain (flashdisk,
  cloud) lewat *Simpan backup ke lokasi lain*, terutama setelah tutup buku.
- Data ada di `%APPDATA%\id.akuntansi.app\data.db`.

## Pintasan keyboard
Tekan `?` di aplikasi untuk daftar lengkap. Yang utama: `Ctrl+K` palet perintah (cari halaman, akun, jurnal),
`Alt+1…8` pindah halaman, `Alt+N` jurnal baru, `Ctrl+S` simpan, `Ctrl+Enter` posting.

## Pembaruan
Aplikasi memberi tahu bila ada versi baru, tetapi **tidak pernah memasangnya sendiri**. Pilih "Lihat" untuk membaca
catatan rilis lalu "Pasang sekarang" saat Anda siap. Data dibackup otomatis sebelumnya.

## Persyaratan sistem
**Windows 11**, 64-bit, dengan **Microsoft Edge WebView2 Runtime** (sudah termasuk di Windows 11; bila tidak ada,
installer mengunduhnya otomatis dan butuh internet hanya saat memasang). Windows 10 kemungkinan besar bisa, tetapi
**belum diuji**.

## Jika Windows menampilkan peringatan saat memasang
Versi beta ini belum memakai sertifikat tanda tangan Windows (code signing), jadi saat membuka installer Windows
SmartScreen menampilkan **"Windows protected your PC"** dengan penerbit "Unknown publisher". Itu normal untuk
aplikasi baru yang belum bertanda tangan, bukan tanda aplikasinya berbahaya. Untuk melanjutkan:
1. Klik **More info**.
2. Klik **Run anyway**.

Agar yakin berkasnya asli: unduh **hanya** dari halaman Releases resmi proyek ini, dan cocokkan nilai **SHA-256**
yang tercantum di catatan rilis dengan hasil perintah ini di PowerShell:
```powershell
Get-FileHash .\Akuntansi_0.1.0_x64-setup.exe -Algorithm SHA256
```
Bila nilainya berbeda, jangan dipasang. Pembaruan di dalam aplikasi tidak terpengaruh peringatan ini, karena
paketnya diverifikasi dengan tanda tangan terpisah (kunci updater) sebelum dipasang.
