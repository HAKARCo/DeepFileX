# 🔷 DeepFileX

**DeepFileX** - Ultra-Lightweight Desktop File **Contents** Search & Analysis Solution

> **Latest**: v3.3.0 (2026-07-21)  
> [![Latest Release](https://img.shields.io/github/v/release/HAKARCo/DeepFileX)](https://github.com/HAKARCo/DeepFileX/releases)  
> [![License](https://img.shields.io/badge/license-Proprietary-red.svg)](LICENSE.txt)

---

## 🎯 Overview

**DeepFileX** is a high-performance desktop search utility written in **Rust**. It is designed to scan, index, and retrieve not only **filenames** but also the **full-text content** of documents across your local storage. 

By building a secure local database index using parallel multi-threaded scanning and SQLite FTS5, DeepFileX allows you to search through the actual contents of PDFs, HWP/HWPX, Office documents, code files, and text files in milliseconds—completely offline, ensuring 100% data privacy.

---

## 🚀 Quick Start

### Build & Run from Source

1. Ensure you have the Rust toolchain installed.
2. Navigate to the `hakar-core` directory and run:
   ```bash
   # Run in developer mode
   cargo run
   ```
3. To compile the optimized, standalone release binary:
   ```bash
   # Compile optimized release binary
   cargo build --release
   ```
4. Find the standalone executable at: `target/release/DeepFileX.exe`

---

## ⭐ Core Features

### 🔒 Real-Time Blackbox JSON Diagnostic Logger
- **Append-Only Diagnostic Log**: Appends all user & system events in real-time to `%USERPROFILE%\Documents\DeepFileX\Logs\blackbox_log.json` without overwriting historical logs.
- **Rich Diagnostic Metadata**: Captures unique UUID `session_id`, `datetime` timestamps (`YYYY-MM-DD HH:MM:SS.mmm`), and formatted execution duration (`"145ms"`, `"5.981s"`).
- **Lifecycle Tracking**: Automatically logs application `startup` and `shutdown` events (`eframe::App::on_exit`).

### ⚡ 2-Phase Engine Architecture & Performance
- **Phase 1 (MFT Filename Scan)**: Reads 100% of all file and folder entries (including `.mp4`, `.exe`, `.zip`, `.pdf`, `.hwp`) for Everything-level sub-millisecond filename searching.
- **Phase 2 (Content & AI Dispatcher)**: Early bypass (100% Skip) for non-document files before reaching heavy parsing pipelines, protecting CPU & RAM footprint by >90%.
- **B-Tree SQL Query Optimization**: High-speed indexing queries without function wrapper overhead (`REPLACE(LOWER(...))`), cutting query response time to sub-millisecond speeds.

### 📄 Native High-Performance Parsers
- **Native HWP / HWPX Parser**: Built-in OLE2, Zstd, and XML text extraction natively compiled into core engine without requiring external DLL plugins.
- **Embedded Pdfium Engine**: Integrates Google Chromium's official Pdfium parser via static bundling (`include_bytes!`), achieving 99% accurate PDF text extraction.
- **SQLite FTS5 + Compression**: Ultra-fast content lookup paired with Zstd Lv3 text compression for minimal disk footprint.

---

## 📁 Supported File Formats & Search Level

DeepFileX employs a hybrid indexing strategy: dedicated high-performance parsers for primary document types, and an optimized binary strings extractor as a universal fallback.

| Category | Extensions | Search Level / Implementation |
| :--- | :--- | :---: |
| **📄 PDF Documents** | `.pdf` | **Full-Text** (Dedicated Embedded Pdfium) |
| **🇰🇷 HWP / HWPX** | `.hwp`, `.hwpx` | **Full-Text** (Native Core OLE2 / Zstd / XML Engine) |
| **📁 Word / Excel / PPT** | `.docx`, `.xlsx`, `.pptx` | **Full-Text** (Dedicated XML/Zip Parsers) |
| **⚙️ Configs & Scripts** | `.yaml`, `.yml`, `.ini`, `.cfg`, `.conf`, `.toml`, `.env`, `.properties`, `.txt`, `.csv`, `.log`, `.srt`, `.vtt`, `.md`, `.json`, `.xml` | **Full-Text** (Dedicated Text Parser) |
| **💻 Code Files** | `.py`, `.js`, `.java`, `.c`, `.cpp`, `.h`, `.cs`, `.rs`, `.go`, `.sh`, `.bat`, `.html`, `.css` | **Full-Text** (Dedicated Text Parser) |
| **🔌 Dynamic Plugins** | `.dwg`, `.dxf` | **Full-Text** (Optional Dynamic CAD DLL Plugin via `🔌 Plugins Manager`) |
| **🖼️ Media & Archives** | `.zip`, `.png`, `.jpg`, `.mp4`, `.avi`, `.dat`, `.bin`, `.dll`, `.exe`, etc. | *Filename Search Only* (Phase 2 Early Bypass) |

---

## 💾 System Requirements

- **OS**: Windows 10 / 11 (64-bit)
- **RAM**: 4GB+ recommended
- **Storage**: Less than 10MB (Single binary size: ~4.5MB)
- **Rust Toolchain**: 1.75+ (Developer compilation only)

---

## 📊 Performance Metrics

- **First Scan Speed (MFT)**: Under 5 seconds for 1.8 Million files
- **Search Response Time**: Sub-millisecond (~0.1ms)
- **Idle Memory Footprint**: Under 10MB
- **Active Scan Memory Footprint**: Under 30MB
- **Standalone Binary Size**: ~4.5MB (LTO & Strip optimized)

---

## 📈 Recent Updates

### v3.3.0 (2026-07-21)
- 🔒 **Real-Time Blackbox JSON Logger**: Added append-only logging to `%USERPROFILE%/Documents/DeepFileX/Logs/blackbox_log.json` with UUID session tracking, datetime, duration, and 100% English GUI.
- ⚡ **2-Phase Engine Architecture**: Implemented Phase 1 MFT 100% filename scan + Phase 2 Content/AI Dispatcher 100% early skip for non-document files.
- 🚀 **B-Tree SQL Query Optimization**: Removed function wrappers (`REPLACE(LOWER(...))`) on external DB queries for instant B-Tree search responses.
- 🔍 **Smart Loading Indicator**: Display `Searching index databases...` loading spinner only after typing non-empty search queries before results render.
- ⚙️ **Dedicated Update Check Dialog**: Separated `⚙️ Settings...` from `🔄 Check for Updates` in top menu.
- 🔌 **Streamlined Plugin Manager**: Removed HWP/HWPX from Plugin Manager UI since HWP/HWPX parsing is 100% natively built into core engine.

📖 See [CHANGELOG.md](CHANGELOG.md) for complete historical details.

---

## 🛠️ Development & Build Guide

### Directory Structure

```
hakar-core/
├── src/                  # Rust source code
│   ├── main.rs           # GUI Application Entry (egui)
│   ├── parser.rs         # File text extractors (Pdfium, HWP, zip, xml)
│   ├── db.rs             # SQLite FTS5 database mapping
│   ├── ntfs.rs           # NTFS MFT parser
│   ├── blackbox/         # Real-time JSON Blackbox Logger subsystem
│   └── update/           # Auto-updater subsystem
├── Cargo.toml            # Dependencies and Release profiles
├── build.rs              # Windows resource compiler (icon mapping)
└── README.md             # Project documentation
```

### Build Command

To compile the production-ready optimized executable, run:
```bash
cargo build --release
```
The output binary will be generated at `target/release/DeepFileX.exe`.

---

## 🤝 Contributing

This is a proprietary utility belonging to **HAKAR**. External contributions require signing an NDA.
