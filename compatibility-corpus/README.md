# compatibility-corpus

Phase corpus manifests for Ubuntu v1 shipment and later Hancom-grade compatibility work.

## Manifests

- `phase1-supported.tsv`
  Safe documents that must parse, save, and reparse successfully.
- `phase1-protected.tsv`
  Failure-scenario corpus. Entries can require `protected-view` with stable issue codes, or an
  explicit `parse-error` for malformed fixtures.
- `phase2-extended.tsv`
  Broader exploratory corpus for later promotion work and compatibility report artifacts.
- `hancom-quality-manifest.tsv`
  한컴 호환 품질 매니페스트. 샘플별 예상 editMode, 페이지 수, 레이아웃 허용오차,
  저장 후 재파싱 방식, Web/Ubuntu/Windows 게이트를 추적한다.
- `hancom-shortcut-audit.tsv`
  한컴 기본 단축키와 RHWP 커맨드/컨텍스트/테스트 연결 상태를 추적한다.
- `hancom-feature-parity-matrix.tsv`
  파일/편집/보기/입력/서식/쪽/표/검토/보안/플랫폼 기능 축의 한컴 대비 구현 상태를
  `implemented | partial | stub | unsupported | missing | bug` 기준으로 추적한다.

## TSV format

Columns are tab-separated.

1. `path`
2. `expected_edit_mode`
3. `expected_save_format`
4. `required_issue_codes`
5. `roundtrip_mode`

Blank columns are allowed.

Valid values:

- `expected_edit_mode`: `editable-safe`, `protected-view`, `parse-error`
- `expected_save_format`: `hwp`, `hwpx`, `unknown`
- `required_issue_codes`: comma-separated stable compatibility codes, or `none`
- `roundtrip_mode`: `save-reparse`, `none`

`parse-error` entries skip compatibility-report generation and pass when `compat-corpus` fails
while parsing or rendering the fixture.

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

Generate JSON diagnostics for CI/snapshot tooling:

```powershell
cargo run --bin rhwp -- compat-report --json compatibility-corpus/fixtures/basic-shape.hwpx
```

Emit per-entry JSON reports while validating a manifest:

```powershell
cargo run --bin rhwp -- compat-corpus --json --emit-reports compatibility-reports/phase2 compatibility-corpus/phase2-extended.tsv
```

Wave 2 shape promotion is finally gated by the Windows Hancom manual checklist in [`WAVE2_WINDOWS_HANCOM_CHECKLIST.md`](./WAVE2_WINDOWS_HANCOM_CHECKLIST.md).
