# Power Paste

Power Paste is a desktop clipboard history manager built with `Tauri 2`, `Vue 3`, and `Rust`. It is designed around a native-feeling workflow: monitor clipboard changes in the background, open a compact history panel with a global shortcut, then quickly search, preview, copy, edit, or paste previous items back into the last target application.

It is not only a utility that gets the job done. Power Paste is also built as a polished desktop product: a translucent panel, light and dark themes, accent colors, and a compact visual language that aims to make a high-frequency productivity tool feel refined enough to keep open every day.

中文说明见 [README.zh-CN.md](./README.zh-CN.md)。

## Product Preview

| Main Panel (Light) | Main Panel (Dark) | Setting |
|---|---|---|
| ![Power Paste light theme](./docs/light.png) | ![Power Paste dark theme](./docs/dark.png) |![Power Paste settings panel](./docs/settings.png)|

## Why Power Paste

- Fast: open the panel with a global shortcut and bring previous clipboard content back in seconds
- Native-feeling: designed around desktop workflows instead of browser-like interaction patterns
- Good-looking: translucent surfaces, theme switching, and accent colors are part of the product value
- Meant to stay around: tray support, single-instance behavior, and update checks make it practical as an always-available companion

## Highlights

- Global shortcut to toggle the history panel
- Capture text, image, and mixed clipboard content
- Detect copied links and open them in the default browser from the history item
- Search and filter by `All`, `Pinned`, `Text`, `Image`, and `Image + Text`
- Pin important entries to keep them at the top
- Edit plain-text history items in place
- Restore clipboard content or paste directly back to the previous target app when supported on the current platform
- Hover image thumbnails to preview a larger image
- Settings for language, theme mode, accent color, launch on startup, history size, image size, debug mode, and global shortcut
- Tray integration, single-instance behavior, startup update checks, and manual update checks from the tray menu
- Local persistence powered by SQLite plus on-disk image storage

## Platform Status

- Windows: primary target platform, with full clipboard monitoring, clipboard write-back, direct paste, autostart, tray integration, updater, and global shortcut workflow
- macOS: clipboard monitoring, clipboard write-back, and direct paste are supported, but direct paste depends on Accessibility / Automation permission from the system
- Linux: clipboard monitoring, clipboard write-back, history browsing, tray integration, updater checks, global shortcut workflow, and launch on startup are supported; direct paste is supported in X11 sessions with `xdotool`, while Wayland remains in explicit degraded mode

## Feature Overview

### History Workflow

- The main panel opens as a compact transparent window
- Arrow keys move through the filtered list and keep the active item in view
- `Enter` pastes the selected item back to the last target application when supported
- `Ctrl/Cmd + C` copies the selected history item back to the system clipboard
- Double-clicking a history item pastes it directly when direct paste is available
- Link items can show an open-link action in the bottom-right corner

### Item Types

- Text items: searchable, editable, copyable, and directly pasteable on supported platforms
- Link items: detected from copied URLs and openable in the system default browser
- Image items: thumbnail preview, large-image hover preview, copy/paste support on supported platforms
- Mixed items: preserved as combined content where the backend supports mixed replay

### Settings

- Interface language: Simplified Chinese / English
- Theme: Light / Dark / System
- Accent color: Ocean / Amber / Jade / Rose
- Launch on startup
- Maximum history item count
- Maximum stored image size
- Global shortcut recording and clearing
- Debug mode toggle

Update checks are no longer configured from the settings page. The app checks for updates automatically on startup, shows an update icon in the top bar when a new version is available, and also exposes a manual `Check for Updates` action in the tray menu.

### Native Integration

- Single-instance behavior: reuses the existing app instance instead of opening duplicates
- Tray support: keep the app available in the background and trigger `Main Panel` / `Check for Updates` / `Quit`
- Global shortcut registration through Tauri plugin support
- Update checks through the Tauri updater plugin

## Cross-Platform Degradation

The following capabilities remain platform-limited, while macOS direct paste depends on system permission:

- Direct paste on Linux requires an X11 session plus `xdotool`
- Direct paste in Wayland sessions is still unsupported
- Native mixed clipboard replay remains Windows-only; Linux falls back to a single preferred payload when replaying mixed content

History browsing, clipboard monitoring, search, filtering, pinning, editing, deleting, tray usage, update checks, settings persistence, launch on startup, and the general UI remain available on Linux.

## Tech Stack

### Frontend

- `Vue 3`
- `Vite`
- Composition API based composables for state and behavior

### Desktop / Backend

- `Tauri 2`
- `Rust`
- `tauri-plugin-global-shortcut`
- `tauri-plugin-autostart`
- `tauri-plugin-single-instance`
- `tauri-plugin-updater`
- `tauri-plugin-sql` with SQLite
- `tauri-plugin-clipboard-next`

### Windows Integration

- Win32 APIs
- WebView2
- PowerShell-based helpers for Windows-specific clipboard and paste workflows

## Requirements

- Node.js `18+`
- `pnpm` `10+`
- Rust `1.77.2+`

Linux direct paste also requires:

- an X11 desktop session
- `xdotool`

Windows development also requires:

- Windows 10 or Windows 11
- Microsoft WebView2 Runtime

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

Build the frontend:

```bash
pnpm build
```

Run Rust checks:

```bash
cd src-tauri
cargo check
```

Build desktop bundles:

```bash
pnpm tauri build
```

## Data Storage

Application data is stored in the Tauri app-local-data directory.

Typical persisted data includes:

- SQLite history database
- `settings.json`
- captured images on disk

The repository no longer relies on a plain `history.json` file for the primary history store; history is backed by SQLite in the current implementation.

## Project Structure

```text
.
├── src/
│   ├── components/      # Reusable Vue UI pieces
│   ├── composables/     # Frontend state and interaction logic
│   ├── services/        # Tauri invoke/event wrappers
│   ├── styles/          # Shared application styles
│   └── utils/           # Frontend helpers
├── src-tauri/
│   ├── src/commands.rs  # Tauri command entrypoints
│   ├── src/runtime.rs   # Window and runtime behavior
│   ├── src/update.rs    # App updater flow
│   ├── src/repository.rs# SQLite history storage
│   ├── src/storage.rs   # Settings and path storage
│   └── src/clipboard/   # Clipboard backends and platform capabilities
└── scripts/             # Local development helper scripts
```

## Repository Notes

- Package manager: `pnpm`
- Default frontend language in code: JavaScript / Vue SFC
- Native backend language: Rust
- Current default branch in this workspace: `master`

## License

This project is licensed under the GNU Affero General Public License v3.0.

See the [LICENSE](./LICENSE) file for the full license text.

If you modify and deploy this project for users over a network, AGPLv3 requires you to provide the corresponding source code of that modified version to those users.
