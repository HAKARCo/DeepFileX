# 🔷 DeepFileX

**DeepFileX** - File **Contents** Search and Analysis Solution

> **Latest**: v2.2.0 (2026-06-29) 
[![Latest Release](https://img.shields.io/github/v/release/HAKARCo/DeepFileX)](https://github.com/HAKARCo/DeepFileX/releases)
[![License](https://img.shields.io/badge/license-Proprietary-red.svg)](LICENSE.txt)

## 🎯 Overview

**DeepFileX** is a high-performance desktop search utility designed to scan, index, and retrieve not only **filenames** but also the **full-text content** of documents across your local storage. 

By building a secure local database index using parallel multi-threaded scanning, DeepFileX allows you to search through the actual contents of PDFs, Office documents (Word, Excel, PowerPoint), code files, and text files in milliseconds—completely offline, ensuring 100% data privacy.

## 🚀 Quick Start

### Run & Build

1. **Run from source or build binary using PyInstaller**
2. **Run DeepFileX.exe from the dist directory**

For developer mode, see the [Development Guide](#-development-guide).

## ⭐ Core Features

### 🔬 Advanced File Analysis
- **30+ File Format Support**: Documents, code, images, archives
- **Real-time Search**: Search by filename and content
- **Persistent Indexing**: SQLite-based database

### ⚡ Ultra-Fast Performance
- **Multi-threading**: Parallel processing for rapid scanning
- **10,000+ Files/Min**: Handle large directories efficiently
- **Memory Optimized**: Efficient resource usage

### 🎨 Modern UI
- **Light/Dark Mode**: Eye-friendly themes
- **Intuitive Interface**: Easy to use
- **Real-time Progress**: Live operation status

## 📁 Supported File Formats

DeepFileX supports indexing and full-text content searching for a wide range of file extensions.

| Category | Type / Extensions | Search Level |
| :--- | :--- | :---: |
| **📄 Documents** | `.pdf`, `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, `.xls`, `.rtf`, `.tex`, `.odt`, `.ods`, `.odp`, `.pages`, `.numbers`, `.key` | **Full-Text** |
| **🇰🇷 Korean Docs** | `.hwp`, `.hwpx` | **Full-Text** |
| **📧 Emails** | `.pst`, `.eml`, `.msg` | **Full-Text** |
| **⚙️ Configurations** | `.yaml`, `.yml`, `.ini`, `.cfg`, `.conf`, `.toml`, `.env`, `.properties`, `.gitignore`, `.editorconfig` | **Full-Text** |
| **💻 Code & Scripts** | `.py`, `.js`, `.java`, `.c`, `.cpp`, `.h`, `.cs`, `.php`, `.rb`, `.go`, `.rs`, `.swift`, `.kt`, `.sql`, `.ps1`, `.sh`, `.bat`, `.dart`, `.scala`, `.lua`, `.pl`, `.asm` | **Full-Text** |
| **🎨 Web & Templates** | `.html`, `.css`, `.vue`, `.tsx`, `.jsx`, `.scss`, `.less`, `.ejs`, `.pug`, `.hbs`, `.mustache`, `.jinja`, `.twig` | **Full-Text** |
| **📦 Archives** | `.zip`, `.rar`, `.7z`, `.tar`, `.gz`, `.bz2`, `.xz`, `.lz`, `.cab`, `.iso` | *Filename Only* |
| **🖼️ Images & Design** | `.jpg`, `.jpeg`, `.png`, `.gif`, `.bmp`, `.tiff`, `.webp`, `.ico`, `.svg`, `.psd`, `.ai`, `.eps`, `.sketch` | *Filename Only* |

## 💾 System Requirements

- **OS**: Windows 10+ (64-bit)
- **RAM**: 4GB+ recommended
- **Storage**: 100MB+
- **Python**: 3.8+ (developer mode only)

## 📊 Performance Metrics

- Scan Speed: **10,000+ files/minute**
- Search Speed: **Millisecond response time**
- Memory Usage: **~100MB**
- Setup Installer: **51.5 MB (WebEngine excluded)**

## 🔧 Usage

### 1️⃣ File Scanning
1. Click "Scan Folders" to select directory
2. Click "Start Scan" to begin scanning
3. View real-time progress

### 2️⃣ File Search
1. Enter keywords in search box
2. Press Enter or click "Search" button
3. Select file from results to preview

### 3️⃣ Index Management
- **Save Index**: Save current index
- **Load Index**: Load saved index
- **Clear Records**: Reset index

## 📈 Recent Updates

### v2.2.0 (2026-06-29)
- 🔷 Reorganized sidebar filters into clean collapsible categories.
- 🔷 Fixed case sensitivity (Match Case) logic bugs and text position offset.
- 🔷 Added pure black checkbox borders in Light Mode to maximize readability.
- 🔷 Optimized PyInstaller package and Inno Setup installer.

### v1.0.0 (2026-06-13)
- 🔷 Upgraded PyQt6 desktop interface and lightweight file indexer.

📖 See [CHANGELOG.md](CHANGELOG.md) for complete change history.

## 🛠️ Development Guide

### Setup Development Environment

```bash
# 1. Clone repository
git clone https://github.com/HAKARCo/DeepFileX.git
cd DeepFileX

# 2. Create virtual environment (recommended)
python -m venv venv
venv\Scripts\activate  # Windows

# 3. Install dependencies
pip install -r requirements.txt

# 4. Run
python src\deepfilex.py
```

### Building

```bash
# Build executable with PyInstaller
pyinstaller DeepFileX.spec --clean
```

Build output: `dist/DeepFileX_v1.0.0/DeepFileX.exe`

### Project Structure

```
DeepFileX/
├── src/                   # Source code
│   ├── deepfilex.py       # Main application
│   ├── update_checker.py  # Auto-update
│   └── version_config.py  # Version management
├── tests/                 # Test files
├── build/                 # Build configuration
│   ├── specs/             # PyInstaller specs
│   └── scripts/           # Build scripts
├── docs/                  # Documentation
│   ├── releases/          # Release notes
│   └── CONTRIBUTING.md    # Contribution guide
├── README.md              # Project overview
├── CHANGELOG.md           # Change history
├── LICENSE.txt            # Proprietary License
└── requirements.txt       # Dependencies
```

## 🤝 Contributing

Contributions are welcome! Please follow these steps:

1. **Fork** and clone the repository
2. **Create branch**: `git checkout -b feature/AmazingFeature`
3. **Commit changes**: `git commit -m "✨ feat: Add AmazingFeature"`
4. **Push**: `git push origin feature/AmazingFeature`
5. **Create Pull Request**

### Contribution Guidelines

- ✅ Follow PEP 8 coding style
- ✅ Test changes thoroughly
- ✅ Update documentation for new features
- ✅ Maintain Python 3.8+ compatibility
- ✅ Reference related issue numbers in PRs

## 📝 License

This project is licensed under the Proprietary License (All Rights Reserved). See [LICENSE.txt](LICENSE.txt) for details.

## 📞 Support & Contact

- **GitHub**: https://github.com/HAKARCo/DeepFileX
- **Releases**: https://github.com/HAKARCo/DeepFileX/releases
- **Issues**: https://github.com/HAKARCo/DeepFileX/issues
- **Discussions**: https://github.com/HAKARCo/DeepFileX/discussions

---

**DeepFileX v1.0.0** by **QuantumLayer** - Advanced File Analysis System 🔷
