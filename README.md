<p align="center">
  <img src="assets/DeepFileX-logo.png" alt="DeepFileX logo" width="240">
</p>

<h1 align="center">DeepFileX</h1>

<p align="center">
  Find files by name. Search inside documents in folders you choose.
</p>

<p align="center">
  <a href="https://github.com/HAKARCo/DeepFileX/releases/latest/download/DeepFileX-Setup-x64.exe">Download for Windows</a>
  ·
  <a href="https://github.com/HAKARCo/DeepFileX/releases">All releases</a>
</p>

DeepFileX is a Windows desktop app for finding files by name and searching text inside indexed documents. Filename scan data stays in memory. Content indexes are saved locally on your computer.

## Features

- **Filename search** filters the scanned file list as you type. DeepFileX starts scanning the C: drive when the app opens.
- **Content search** builds a searchable text index for a folder you choose. Select saved indexes in the sidebar to choose where to search.
- **Matching results** show file details in both search modes. Open a result's location from its context menu.
- **Local feedback log** can be exported from the Help menu for review before sharing.

## Get started

1. Download and run **DeepFileX-Setup-x64.exe**.
2. Open **DeepFileX** from the Start menu and approve the Windows administrator prompt needed for NTFS scanning.
3. In **Filename**, wait for the initial scan to finish, then type in the search box.
4. In **Content**, choose a folder and build its index. Check its saved index in the sidebar, then search its document text.

## Requirements and supported content

- Windows 10 or 11, 64-bit.
- A local NTFS drive for filename scanning.
- Content search supports TXT, CSV, Markdown, common code and configuration formats, DOCX, XLSX, PPTX, and PDFs with extractable text.

Content search uses the saved index, so results may not reflect later file changes. Rescan the folder to update its index. PDFs that contain only scanned images need OCR before their text can be searched.

## Updates and feedback

Use **Help → Check for updates** to check GitHub Releases. Use **Help → Export feedback log…** to save a local log you can review before sharing. Report issues on the [GitHub issue tracker](https://github.com/HAKARCo/DeepFileX/issues).
