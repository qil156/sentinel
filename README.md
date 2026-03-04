# Sentinel

Sentinel is an MVP desktop AI assistant for Windows that answers questions about the current foreground window.

## What It Does

1. The user asks a question in a floating chat widget.
2. The backend captures the current foreground window.
3. The app sends the question, screenshot, and window title to the OpenAI Responses API.
4. The UI renders the structured JSON result.

## Project Layout

- `src-tauri/`: Rust backend and Tauri shell
- `src-tauri/src/platform/`: OS-specific capture implementations (`windows.rs`, `macos.rs`)
- `ui/`: React frontend
- `shared/`: shared TypeScript types

## Environment

Set `OPENAI_API_KEY` before launching the Tauri app.

PowerShell:

```powershell
$env:OPENAI_API_KEY="sk-..."
```

## Run

From `sentinel/ui`:

```powershell
npm install
```

From `sentinel/src-tauri`:

```powershell
cargo tauri dev
```

## Notes

- This MVP is read-only. It never clicks, types, or performs OS actions.
- Windows capture uses `GetForegroundWindow` and `GetWindowRect`.
- Screenshot capture grabs the containing display, then crops to the foreground window bounds.
- Multi-monitor handling is basic but functional for the common case where the foreground window is mostly on one display.
- `src-tauri/src/platform/macos.rs` is a placeholder for future macOS implementation.
