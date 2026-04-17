# Wave 5 Metadata Preservation

- Date: 2026-04-17
- Scope: preserved HWPX metadata that remains editable-safe while untouched

## Covered issue codes

- `hwpx-docinfo-extra-records`
- `hwpx-section-page-border-fill`
- `hwpx-section-master-pages`
- `hwpx-section-extra-records`
- `hwpx-unknown-control`

## Regression anchors

- Snapshot: `preserved_wave5_metadata_compatibility_report`
- Snapshot: `preserved_unknown_control_compatibility_report`
- JSON baseline: `metadata-compat-reports.json`
- Fixture report: `unknown-control-fixture.json`
- Serializer test: `test_preservation_context_downgrades_wave5_metadata_until_dirty`
- DocumentCore test: `compatibility_report_marks_dirty_wave5_metadata_as_protected`
- DocumentCore test: `compatibility_report_marks_dirty_unknown_control_as_protected`
- Parser test: `test_parse_unknown_empty_control_inside_ctrl`

## Policy

- Clean snapshot-backed metadata is downgraded to `warning` and stays `editable-safe`.
- Dirty docinfo or dirty sections elevate the same issue codes back to `blocker`.
- Unknown controls remain warning-only while the containing section stays untouched.

## Corpus anchor

- Fixture: `compatibility-corpus/fixtures/unknown-control.hwpx`
- Manifest: `compatibility-corpus/phase2-extended.tsv`
- Expected outcome: `editable-safe` with `hwpx-unknown-control`
