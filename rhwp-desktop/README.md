# rhwp-desktop

Ubuntu desktop wrapper for `rhwp-studio`.

Current phase-1 scope:

- Load the built `rhwp-studio` frontend inside Tauri.
- Open `.hwp` / `.hwpx` files through native dialogs.
- Save with original extension policy from the studio session.
- Persist recent documents in the app data directory.
- Emit `rhwp://open-files` when startup files are passed on launch.
- Package an Ubuntu `.deb` with desktop and MIME registration assets.

Expected flow:

1. Build the WASM package in the repository root.
2. Build `rhwp-studio`.
3. Run `npm install` inside `rhwp-desktop`.
4. Run `npm run dev` or `npm run build`.

Ubuntu packaging assets live under [packaging/linux](./packaging/linux).

Ubuntu package build:

1. On Ubuntu 22.04 or newer, install the Tauri Linux prerequisites.
2. Run `npm run build:linux` inside `rhwp-desktop`.
3. Pick up the generated `.deb` from `src-tauri/target/release/bundle/deb/`.

GitHub Actions:

- `.github/workflows/rhwp-desktop-linux.yml` builds the Ubuntu `.deb` on `ubuntu-22.04`.
- The same workflow uploads the package as the `rhwp-ubuntu-deb` artifact.
- A follow-up smoke-test job installs the built package on `ubuntu-24.04` and verifies the desktop and MIME files are present.
