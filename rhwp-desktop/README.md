# rhwp-desktop

Ubuntu desktop wrapper for `rhwp-studio`.

Current phase-1 scope:

- Load the built `rhwp-studio` frontend inside Tauri.
- Open `.hwp` / `.hwpx` files through native dialogs.
- Save with original extension policy from the studio session.
- Persist recent documents in the app data directory.
- Emit `rhwp://open-files` when startup files are passed on launch.

Expected flow:

1. Build the WASM package in the repository root.
2. Build `rhwp-studio`.
3. Run `npm install` inside `rhwp-desktop`.
4. Run `npm run dev` or `npm run build`.

Ubuntu packaging assets live under [packaging/linux](./packaging/linux).
