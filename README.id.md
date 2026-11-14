<div align="center">

# Muse Desktop

### Muse kamu, sekarang punya tangan di PC Windows kamu sendiri — dengan aman.

Pendamping desktop ringan dan terbuka yang memungkinkan **agen Muse
kamu sendiri** bekerja di **satu folder pilihanmu** di PC Windows kamu.

[![Build](https://github.com/machiavellia-lynn/muse-desktop/actions/workflows/build-windows.yml/badge.svg)](https://github.com/machiavellia-lynn/muse-desktop/actions/workflows/build-windows.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)
![Built with Tauri + Rust](https://img.shields.io/badge/built%20with-Tauri%20%2B%20Rust-orange)

[English](README.md) · **Bahasa Indonesia**

</div>

> [!WARNING]
> **Ini proyek komunitas TIDAK RESMI.** Bukan buatan Meta dan tidak
> berafiliasi dengan Meta. “Muse” adalah merek dagang Meta. Aplikasi
> ini hanya membuka situs resmi [muse.ai](https://muse.ai) milik Meta
> dalam sebuah jendela dan menambahkan bridge lokal bersandbox di
> mesin kamu sendiri. Seluruh source code-nya ada di sini — baca dulu
> sebelum percaya.

---

## ⬇️ Download

### [⬇️ DOWNLOAD — Muse-Desktop-Windows.zip](https://nightly.link/machiavellia-lynn/muse-desktop/actions/runs/43485913737/Muse-Desktop-Windows.zip)

Satu klik, tanpa login. Unzip — isinya dua file ini:

| File | Isi |
|---|---|
| `Muse Desktop_0.2.0_x64-setup.exe` | Installer (disarankan) |
| `muse-desktop.exe` | Portable — tanpa install, langsung jalan |

*(Link langsung ke build v0.2.0 saat ini. Build yang lebih baru
muncul di halaman **Releases** / **Actions** repo.)*

> Windows mungkin menampilkan peringatan SmartScreen karena aplikasi
> ini belum ditandatangani secara digital. Klik **More info → Run
> anyway** — atau build sendiri dari source ini (cara di bawah) supaya
> kamu tidak perlu percaya binary siapa pun.

**Yang kamu butuhkan:**
- ✅ PC Windows 10/11
- ✅ Akun Muse (login pakai akun kamu sendiri)
- ✅ Itu saja — pilih **Mode Mudah** (bawaan, tanpa akun) atau jalur
  **Tailscale** yang lebih privat (akun gratis) di langkah 3
- ⏱️ Sekitar 10 menit, sekali saja

---

## 🚀 Instalasi 4 langkah

**1. Instal aplikasinya**
Jalankan installer (atau `.exe` portable). Dua jendela terbuka:
jendela utama **Muse** dan **Control Panel** kecil.

**2. Login**
Di jendela utama, login ke muse.ai pakai akun Muse kamu —
persis seperti di browser. Aplikasi ini tidak pernah melihat
password kamu.

**3. Pilih mode koneksi**

*Mode Mudah (disarankan untuk kebanyakan orang)* — di Control Panel,
klik **Aktifkan Mode Mudah**. Setelah ±30 detik panel menampilkan
alamat sementara seperti `https://kata-acak.trycloudflare.com`.
Tanpa akun, tanpa instal tambahan. Alamat berubah tiap aplikasi
dinyalakan ulang; perlakukan seperti kata sandi.

*Mode Privat (Tailscale)* — instal
[Tailscale](https://tailscale.com/download), login, biarkan berjalan,
dan minta Muse kamu bergabung ke jaringanmu (panduan:
**[SETUP.md](SETUP.md)**). Lebih privat: bridge tidak pernah
terekspos ke internet publik sama sekali.

**4. Pasangkan**
Berikan ke Muse kamu dua hal dari Control Panel: **alamat** (alamat
Mode Mudah, atau `http://100.x.x.x:8765` di Tailscale) dan **token
pairing** — bersama prompt siap pakai di
[client/PROMPT-FOR-YOUR-MUSE.md](client/PROMPT-FOR-YOUR-MUSE.md).

Selesai. Coba minta ke Muse kamu:

> “Tampilkan daftar file di workspace laptopku.”

---

## ✨ Yang bisa dilakukan Muse kamu

- 📁 Melihat, membaca, dan membuat file — **hanya** di satu folder
  workspace pilihanmu
- 📝 Menyimpan ringkasan, draft, kode, dan laporan langsung ke PC kamu
- 🖥️ Menjalankan perintah terminal — **hanya** jika kamu
  mengaktifkannya, dan **hanya** setelah kamu klik *Allow* di dialog
  di layarmu sendiri, setiap kali

## 🔒 Yang tidak akan pernah bisa dilakukan

- ❌ Menyentuh apa pun di luar folder pilihanmu (jalan keluar path dan
  trik symlink ditolak di kode, teruji oleh test)
- ❌ Menjalankan perintah diam-diam — jalur eksekusi jarak jauh tanpa
  persetujuan memang tidak dibuat, by design
- ❌ Melihat password atau akun Muse kamu — login terjadi di halaman
  asli Meta; aplikasi tidak menyuntikkan apa pun dan jendela itu
  ditolak semua izin aplikasi
- ❌ Mengirim data ke mana-mana — tidak ada telemetri, tidak ada
  server update, tidak ada cloud pihak ketiga sama sekali

Setiap akses tercatat langsung di Control Panel. Menutup aplikasi
dari tray mematikan bridge sepenuhnya.

---

## 🧩 Cara kerja

```
┌─ PC Windows kamu ─────────────────────────────┐
│  Muse Desktop (satu proses Rust kecil)         │
│  ├─ Jendela 1 · muse.ai (resmi, oleh Meta)    │
│  │             tanpa izin aplikasi, tanpa skrip│
│  ├─ Jendela 2 · Control Panel (halaman lokal) │
│  │             folder · token · log · setujui  │
│  ├─ Ikon tray · buka / panel / keluar         │
│  └─ Bridge · HTTP hanya di IP Tailscale kamu  │
└──────────────────┬────────────────────────────┘
                   │  Tailscale — terenkripsi,
                   │  jaringan privat kamu
┌──────────────────▼────────────────────────────┐
│  Muse kamu · komputer cloud-nya sendiri      │
│  bicara ke bridge pakai token pairing kamu    │
└───────────────────────────────────────────────┘
```

Inti bridge hanya ~700 baris Rust (`crates/muse-bridge`,
hanya std + serde_json) dengan unit test untuk sandbox, pemeriksaan
token, dan alur persetujuan: `cargo test -p muse-bridge`.

Model ancaman lengkap: **[SECURITY.md](SECURITY.md)**.

---

## ❓ FAQ

**Ini aplikasi Muse resmi untuk Windows?**
Bukan. Meta tidak menyediakan aplikasi Windows — Muse ada di web,
iOS, Android, dan Mac. Ini proyek open-source independen (MIT) yang
membuka situs resminya dalam jendela dan menambahkan bridge lokal.

**Amankah?**
Nilai sendiri: source-nya kecil dan terbuka penuh; terminal mati
secara default dan butuh klik kamu per perintah; bridge hanya
mendengarkan di jaringan privat Tailscale kamu dan mewajibkan token
yang hanya kamu yang bisa melihatnya. Daftar keterbatasan yang jujur
ada di [SECURITY.md](SECURITY.md).

**Kenapa Tailscale?**
Muse kamu tinggal di komputer cloud-nya sendiri — ia butuh *suatu*
jalan privat ke PC kamu. Tailscale gratis untuk pemakaian pribadi,
terenkripsi, dan bekerja tanpa membuka port apa pun di router kamu.
Bridge menolak mendengarkan di jalur lain.

**Folder apa yang sebaiknya dipilih jadi workspace?**
Folder khusus (default-nya `C:\Users\<kamu>\MuseWorkspace`).
Jangan arahkan ke seluruh drive, Documents, atau apa pun berisi
rahasia yang tidak akan kamu berikan langsung ke Muse kamu.

**Bisa cabut aksesnya?**
Seketika: keluar dari aplikasi lewat tray. Permanen: hapus
`token.txt` di folder data aplikasi (token baru dibuat saat start
berikutnya) dan/atau hapus perangkat `muse` dari halaman admin
Tailscale kamu.

**macOS / Linux?**
Belum. Inti bridge adalah Rust netral-platform; build Tauri untuk
sistem lain silakan dikontribusikan.

---

## 🛠️ Build dari source

**Cara mudah (tanpa tool di PC kamu):** fork repo ini → workflow
**Build Muse Desktop (Windows)** di GitHub Actions akan membuatkan
installer dan exe portable untukmu, menjalankan test bridge lebih
dulu.

**Lokal di Windows:**

```powershell
# butuh: Rust (rustup) + Node.js 22
npm install
python scripts\make_icon.py   # pip install pillow dulu
npx tauri icon src-tauri\icons\icon.png
npx tauri build
```

## 📂 Isi repo

| Path | Isi |
|---|---|
| `crates/muse-bridge/` | Inti bridge (Rust murni) + semua test |
| `src-tauri/` | Shell Tauri — jendela, tray, dialog persetujuan, izin |
| `src/` | Control Panel (HTML/CSS/JS polos) |
| `client/` | Client Python kecil + prompt pairing untuk Muse kamu |
| `SETUP.md` | Panduan koneksi langkah demi langkah |
| `SECURITY.md` | Model ancaman & keterbatasan, jujur |

## 📜 Lisensi

[MIT](LICENSE) — pakai, fork, bagikan. Sertakan pemberitahuan hak
cipta dan disclaimer tidak resmi ini: **ini bukan produk Meta.**

<div align="center">

Dibuat dengan Rust, Tauri, dan ketidakpercayaan yang sehat pada binary tertutup.

</div>
