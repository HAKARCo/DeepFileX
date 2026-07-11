# 🔷 DeepFileX

**DeepFileX** - Ultra-Lightweight Desktop File **Contents** Search & Analysis Solution

> **Latest**: v3.1.0 (2026-07-11)
> [![Latest Release](https://img.shields.io/github/v/release/HAKARCo/DeepFileX)](https://github.com/HAKARCo/DeepFileX/releases)
> [![License](https://img.shields.io/badge/license-Proprietary-red.svg)](LICENSE.txt)

---

## 🎯 Overview

**DeepFileX** is a high-performance desktop search utility written in **Rust**. It is designed to scan, index, and retrieve not only **filenames** but also the **full-text content** of documents across your local storage. 

By building a secure local database index using parallel multi-threaded scanning and SQLite FTS5, DeepFileX allows you to search through the actual contents of PDFs, Office documents, code files, and text files in milliseconds—completely offline, ensuring 100% data privacy.

---

## 🚀 Quick Start

### Build & Run from Source

1. Ensure you have the Rust toolchain installed.
2. Navigate to the project root and run:
   ```bash
   # Run in developer mode
   cargo run
   ```
3. To compile the optimized, standalone release binary:
   ```bash
   # Compile optimized binary
   cargo build --release
   ```
4. Find the standalone executable at: `target/release/DeepFileX.exe`

---

## ⭐ Core Features

### 🔬 High-Performance File Analysis
- **Advanced Format Support**: Full-text indexing for `.pdf`, `.docx`, `.xlsx`, `.txt`, `.csv`, `.log` and configuration/script files.
- **Embedded Pdfium Engine**: Integrates Google Chromium's official Pdfium parser via static bundling (`include_bytes!`), achieving 99% accurate text extraction without external DLLs.
- **Robust Unicode & Space Pathing**: Fixed Windows pathing locks on folders containing spaces or non-ASCII (Korean CJK) characters.

### ⚡ Extreme Performance & Resource Efficiency
- **Fast NTFS MFT Scan**: Rapid MFT parsing (under 0.2s for 1.6M files) with a fallback multi-threaded WalkDir directory scanner.
- **SQLite FTS5 + RAM Search**: Instant content lookup (under 1ms) paired with sub-millisecond in-memory filename searching.
- **Data Compression**: Cache text metadata efficiently using Zstd Lv3 compression to maintain a minimal storage footprint.

### 🎨 Premium & Compact UI
- **Single Executable**: Packs all GUI, database, and PDF engines into a **single binary under 5MB** (uncompressed).
- **Clean Subsystem**: Completely hides the debug CMD console window in release builds for a native, clean GUI experience.
- **Branded Design**: Window title bar and taskbar icons are dynamically bound to the DFX brand logo via runtime pixel injection.

---

## 📁 Supported File Formats & Search Level

DeepFileX employs a hybrid indexing strategy: dedicated high-performance parsers for primary document types, and an optimized binary strings extractor as a universal fallback.

| Category | Extensions | Search Level / Implementation |
| :--- | :--- | :---: |
| **📄 Documents** | `.pdf` | **Full-Text** (Dedicated Embedded Pdfium) |
| **📁 Word/Excel** | `.docx`, `.xlsx` | **Full-Text** (Dedicated XML/Zip Parsers) |
| **⚙️ Configs & Scripts** | `.yaml`, `.yml`, `.ini`, `.cfg`, `.conf`, `.toml`, `.env`, `.properties`, `.txt`, `.csv`, `.log`, `.srt`, `.vtt`, `.md`, `.json`, `.xml` | **Full-Text** (Dedicated Text Parser) |
| **💻 Code Files** | `.py`, `.js`, `.java`, `.c`, `.cpp`, `.h`, `.cs`, `.rs`, `.go`, `.sh`, `.bat`, `.html`, `.css` | **Full-Text** (Dedicated Text Parser) |
| **Fallback Extraction** | `.pptx`, `.ppt`, `.doc`, `.xls`, `.hwp`, `.hwpx`, `.dwg`, `.dxf`, etc. | **Full-Text Fallback** (Universal Binary Strings Extractor) |
| **📦 Dynamic Plugins** | `.hwp`, `.hwpx`, `.dwg`, `.dxf` (Dedicated Parsers) | **Full-Text** (Planned for v3.2 via DLL FFI) |
| **🖼️ Media & Archives** | `.zip`, `.png`, `.jpg`, `.dat`, `.bin`, `.dll`, `.exe`, etc. | *Filename Only* |

---

## 💾 System Requirements

- **OS**: Windows 10+ (64-bit)
- **RAM**: 4GB+ recommended
- **Storage**: Less than 10MB (Single binary size: ~4.46MB)
- **Rust Toolchain**: 1.70+ (Developer compilation only)

---

## 📊 Performance Metrics

- **First Scan Speed (MFT)**: Under 5 seconds for 1.6 Million files
- **Search Response Time**: ~0.1 milliseconds
- **Idle Memory Footprint**: **Under 10MB**
- **Active Scan Memory Footprint**: **Under 30MB**
- **Standalone Binary Size**: **~4.46MB** (LTO & Strip optimized)

---

## 🔧 Usage

### 1️⃣ File Scanning
1. Open the application.
2. Enter the path in the "Scan Folder" field or select it via the native directory dialog.
3. Click "Scan Folder" to begin the indexing process.

### 2️⃣ Fast Search
1. Type search queries into the main input field.
2. Press Enter or click the Search button to view matching filenames or content snippets instantly.
3. Select any file from the results table to preview details and context matches in the sidebar.

### 3️⃣ Index Persistence
- **Save Index**: Commits scanned file paths and compressed FTS5 content to the local `.hidx` index database.
- **Load Index**: Instantly loads the saved metadata from disk to bypass initial drive scanning upon startup.

---

## 📈 Recent Updates

### v3.1.0 (2026-07-11)
- 🔷 Re-engineered entire architecture from Python/PyQt6 to Rust/egui.
- 🔷 Integrated Google's Pdfium engine statically (`include_bytes!`) for accurate PDF text parsing.
- 🔷 Fixed major Windows pathing locks on CJK/space-containing paths.
- 🔷 Replaced `SUBSTR` logic with SQLite `NOT LIKE` prefix queries to prevent database clean loss.
- 🔷 Hid the development CMD console window in release builds.
- 🔷 Applied custom DFX branding icon dynamically to window viewport and taskbar.

📖 See [CHANGELOG.md](CHANGELOG.md) for complete historical details.

---

## 🛠️ Development & Build Guide

### Directory Structure

```
hakar-core/
├── src/                  # Rust source code
│   ├── main.rs           # GUI Application Entry (egui)
│   ├── parser.rs         # File text extractors (Pdfium, zip, xml)
│   ├── db.rs             # SQLite FTS5 database mapping
│   ├── ntfs.rs           # NTFS MFT parser
│   └── dfx_logo.png      # Brand logo image asset
├── Cargo.toml            # Dependencies and Release profiles
├── build.rs              # Windows resource compiler (icon mapping)
└── README.md             # Project documentation
```

### Build Command

To rebuild the production-ready optimized executable, run:
```bash
cargo build --release
```
The output binary will be written to `target/release/DeepFileX.exe`.

---

## 🤝 Contributing

This is a proprietary utility belonging to **HAKAR**. External contributions require signing a NDA.
