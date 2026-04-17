# compatibility-corpus

Phase corpus manifests for Ubuntu v1 shipment and later Hancom-grade compatibility work.

## Manifests

- `phase1-supported.tsv`
  Safe documents that must parse, save, and reparse successfully.
- `phase1-protected.tsv`
  Risky documents that must open in `protected-view` with stable issue codes.
- `phase2-extended.tsv`
  Broader exploratory corpus for later promotion work and compatibility report artifacts.

## TSV format

Columns are tab-separated.

1. `path`
2. `expected_edit_mode`
3. `expected_save_format`
4. `required_issue_codes`
5. `roundtrip_mode`

Blank columns are allowed.

Valid values:

- `expected_edit_mode`: `editable-safe`, `protected-view`
- `expected_save_format`: `hwp`, `hwpx`, `unknown`
- `required_issue_codes`: comma-separated stable compatibility codes, or `none`
- `roundtrip_mode`: `save-reparse`, `none`

Relative paths are resolved from the manifest directory.

## Fixture generation

Use the built-in CLI to regenerate synthetic HWPX fixtures:

```powershell
cargo run --bin rhwp -- compat-generate-fixtures compatibility-corpus/fixtures
```

Validate a manifest:

```powershell
cargo run --bin rhwp -- compat-corpus compatibility-corpus/phase1-supported.tsv
```

Generate a structured report for one document:

```powershell
cargo run --bin rhwp -- compat-report compatibility-corpus/fixtures/phase1-basic-text.hwpx
```
