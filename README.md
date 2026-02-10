# Easy Call AI

M1 skeleton based on `Tauri 2 + Rust + Vue 3`.

## Current Capabilities
- Global hotkey `Alt+C` (show main window)
- System tray menu (`Show` / `Hide` / `Quit`)
- Config load/save via Tauri commands
- Frontend settings panel wired to Rust backend
- `rig` provider path wired (`chat_with_rig` using OpenAI-compatible config)
- Debug provider config at `.debug/api-key.json` (OpenAI-compatible schema)
- Fixed text probe command (`send_debug_probe`) for cache-hit testing

## Project Structure
- `src/`: Vue frontend
- `src-tauri/`: Rust backend and Tauri config
- `plan/plan.md`: development roadmap

## Run (Windows)
1. Install dependencies:
```bash
pnpm install
```
2. Start app in Tauri dev mode:
```bash
pnpm tauri dev
```

## Notes
- In M1, `apiKey` is still stored in config file for faster integration.
- We will move key storage to `keyring` in the next milestone.
- If `.debug/api-key.json` exists and `enabled=true`, it takes precedence over UI settings.

