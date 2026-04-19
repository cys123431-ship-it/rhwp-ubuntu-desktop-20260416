# rhwp-desktop

Desktop wrapper for `rhwp-studio` on Ubuntu and Windows.

## Desktop scope

- Package `rhwp-studio` as a Tauri desktop app for Ubuntu `22.04/24.04 LTS x86_64` and Windows `10/11 x64`.
- Open `.hwp` and `.hwpx` through native file dialogs, startup file arguments, and desktop file associations.
- Keep the original extension on `Save`; allow format conversion on `Save As`.
- Show a first-run banner when `rhwp` is not yet the default app for HWP/HWPX documents.
- Persist recent documents and automatic recovery snapshots in the app data directory.
- Block risky documents with `protected-view` instead of attempting lossy saves.

## Install and first-run flow

### Ubuntu

1. Install the generated `.deb`.
2. Launch `rhwp`.
3. If the session banner offers `Set as default app`, click it once.
4. After that, `.hwp` and `.hwpx` files can be opened by double-click in Nautilus.

The app uses `xdg-mime default rhwp.desktop application/x-hwp application/x-hwpx` for user-level default-app registration.

### Windows

1. Install the generated NSIS `.exe` or MSI package.
2. Launch `rhwp`.
3. If the session banner offers `Open Default Apps Settings`, open it once.
4. In Windows Settings, choose `rhwp` as the default app for `.hwp` and `.hwpx`.
5. After that, `.hwp` and `.hwpx` files can be opened by double-click in Explorer.

Windows default-app selection requires explicit user confirmation, so `rhwp` opens Settings instead of changing defaults silently.

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

## Windows package build

1. On Windows, install the Rust stable toolchain, Node.js, and Visual Studio Build Tools with the MSVC/Windows SDK components.
2. Run `npm run build:windows` inside `rhwp-desktop`.
3. Pick up the generated installers from `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/`.
4. Tag builds can sign the installers when `WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD` are configured in GitHub Actions.
5. For local testing without a publicly trusted certificate, use `packaging/windows/sign-release-assets.ps1` to generate a self-signed code-signing certificate, sign the Windows installers, and export the public `.cer` file for manual trust installation.

## CI and releases

- `.github/workflows/rhwp-desktop-linux.yml` builds the Ubuntu `.deb` on `ubuntu-22.04`.
- `.github/workflows/rhwp-desktop-windows.yml` builds the Windows NSIS `.exe` and MSI installers on `windows-latest`.
- The same workflow regenerates synthetic phase-1-safe HWPX fixtures, validates the `phase1-supported` / `phase1-protected` / `phase2-extended` corpus manifests, and uploads per-document `compat-report` artifacts for the phase-2 set before packaging.
- The Linux workflow installs the package on `ubuntu-22.04` and `ubuntu-24.04`, validates the desktop and MIME assets, and runs installed-package WebDriver E2E under `xvfb`.
- The Windows workflow runs NSIS/MSI install smoke tests, validates file-handler registration, and runs installed-package desktop E2E.
- Tag pushes matching `v*` upload both the `.deb` artifact and the Windows installers to the GitHub release automatically.

Corpus manifests live under [`compatibility-corpus/`](../compatibility-corpus/README.md).

## Wave 2 desktop E2E

The installed-package E2E suite lives under [`e2e/`](./e2e/) and exercises the packaged desktop app through `tauri-driver` + `selenium-webdriver`.

The automated desktop flow verifies:

- the first-run default-app banner on Linux and Windows
- startup open of the representative Wave 2 HWPX sample
- recovery snapshot restore and cleanup after save
- one-window-per-file startup fan-out for multiple input documents
- single-instance handoff when a second launch opens another document

Run it locally after installing the package and `tauri-driver`:

1. Install any platform-specific WebDriver prerequisites.
2. Export `RHWP_E2E_APP` if needed.
3. Run `npm ci` in `rhwp-desktop`.
4. Run `npm run e2e:installed`.

## Compatibility roadmap

Phase 1 focuses on desktop installation, file association registration, safe editing, and original-format save behavior.

Hancom-grade compatibility work continues in later phases:

- richer numbering and bullet round-trip
- object and shape support
- field and form preservation
- layout and font-substitution diagnostics
- unsupported-node preservation instead of whole-document fallback to `protected-view`
