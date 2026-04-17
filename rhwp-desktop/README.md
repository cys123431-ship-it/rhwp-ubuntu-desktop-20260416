# rhwp-desktop

Ubuntu desktop wrapper for `rhwp-studio`.

## Phase 1 scope

- Package `rhwp-studio` as a Tauri desktop app for Ubuntu `22.04/24.04 LTS x86_64`.
- Open `.hwp` and `.hwpx` through native file dialogs, startup file arguments, and desktop file associations.
- Keep the original extension on `Save`; allow format conversion on `Save As`.
- Show a first-run banner when `rhwp` is not yet the default app for `application/x-hwp` and `application/x-hwpx`.
- Persist recent documents and automatic recovery snapshots in the app data directory.
- Block risky documents with `protected-view` instead of attempting lossy saves.

## Ubuntu install and first-run flow

1. Install the generated `.deb`.
2. Launch `rhwp`.
3. If the session banner offers `Set as default app`, click it once.
4. After that, `.hwp` and `.hwpx` files can be opened by double-click in Nautilus.

The app uses `xdg-mime default rhwp.desktop application/x-hwp application/x-hwpx` for user-level default-app registration.

## Recovery and save policy

- Recovery snapshots are written for dirty, editable desktop documents.
- A matching recovery snapshot is offered when reopening the same file.
- Untitled recovery snapshots are offered on startup before a blank document is created.
- Saving a document deletes the linked recovery snapshot after a successful write.
- `protected-view` documents remain read-only and cannot be saved until support is implemented.

## Local build

1. Build the WASM package in the repository root.
2. Build `rhwp-studio`.
3. Run `npm install` inside `rhwp-desktop`.
4. Run `npm run dev` or `npm run build`.

Ubuntu packaging assets live under [packaging/linux](./packaging/linux).

## Ubuntu package build

1. On Ubuntu 22.04 or newer, install the Tauri Linux prerequisites.
2. Run `npm run build:linux` inside `rhwp-desktop`.
3. Pick up the generated `.deb` from `src-tauri/target/release/bundle/deb/`.

## CI and releases

- `.github/workflows/rhwp-desktop-linux.yml` builds the Ubuntu `.deb` on `ubuntu-22.04`.
- The same workflow regenerates synthetic phase-1-safe HWPX fixtures and runs the `phase1-supported` / `phase1-protected` corpus manifests before packaging.
- The same workflow installs the package on `ubuntu-24.04`, validates the desktop and MIME assets, configures `xdg-mime`, and launches sample `.hwp` and `.hwpx` files under `xvfb`.
- Tag pushes matching `v*` upload the `.deb` artifact to the GitHub release automatically.

Corpus manifests live under [`compatibility-corpus/`](../compatibility-corpus/README.md).

## Compatibility roadmap

Phase 1 focuses on Ubuntu installation, desktop integration, safe editing, and original-format save behavior.

Hancom-grade compatibility work continues in later phases:

- richer numbering and bullet round-trip
- object and shape support
- field and form preservation
- layout and font-substitution diagnostics
- unsupported-node preservation instead of whole-document fallback to `protected-view`
