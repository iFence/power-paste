# Power Paste

Power Paste is a desktop clipboard history manager built with `Tauri 2`, `Vue 3`, and `Rust`.

## Platform Status

- Windows: full support for clipboard capture, clipboard write-back, direct paste to the last target app, launch on startup, tray integration, and global shortcut workflow
- macOS: app startup and packaging are intended to work, but Windows-native features are currently shown as unsupported
- Linux: app startup and packaging are intended to work, but Windows-native features are currently shown as unsupported

## Current Features

- Clipboard history panel opened with a global shortcut
- Text, image, and mixed clipboard item capture
- Search and filter by item type
- Pin important items to the top
- Edit plain text history items
- Source app detection for captured items
- Theme, accent color, language, and density settings
- Ignored app list for sensitive applications
- Tray integration and single-instance behavior
- Local persistence for history, settings, and captured images

## Cross-Platform Behavior

These features are currently Windows-only and return friendly unsupported messages on macOS and Linux:

- Write item content back to the system clipboard
- Paste directly into the previous target application
- Launch on startup
- Native mixed clipboard replay

Platform-independent features such as history browsing, search, filtering, pinning, editing, deleting, and settings persistence remain available.

## Stack

- `Tauri 2`
- `Vue 3`
- `Vite`
- `Rust`
- Windows native clipboard integration via `PowerShell`, Win32 APIs, and `System.Windows.Forms`

## Requirements

- Node.js 18+
- `pnpm` 10+
- Rust 1.77+

Windows development also requires:

- Windows 10 or Windows 11
- WebView2 Runtime

## Development

Install dependencies:

```bash
pnpm install
```

Run the frontend only:

```bash
pnpm dev
```

Run the Tauri desktop app:

```bash
pnpm tauri dev
```

## Build

Frontend build:

```bash
pnpm build
```

Rust check:

```bash
cd src-tauri
cargo check
```

Desktop package build:

```bash
pnpm tauri build
```

## Data Storage

Power Paste stores local data in the Tauri app-local-data directory.
Typical files include:

- `history.json`
- `settings.json`
- `images/`

## Project Structure

```text
.
- src/                 # Vue UI
- src/components/      # UI components
- src/composables/     # Frontend state and behavior
- src/services/        # Tauri API wrappers
- src/utils/           # Frontend helpers
- src/styles/          # Styles
- src-tauri/src/       # Rust backend
```
