# Wave 3 Field And Char Overlap Preservation

- Date: 2026-04-17
- Scope: preserve-only HWPX field controls without tracked ranges and character overlap controls

## Covered issue codes

- `hwpx-field-missing-range`
- `hwpx-char-overlap`

## Regression anchors

- Snapshot: `preserved_field_overlap_compatibility_report`
- DocumentCore test: `compatibility_report_marks_dirty_field_overlap_as_protected`
- Corpus sample: `../samples/table-vpos-01.hwpx`

## Policy

- Clean snapshot-backed sections downgrade both issue codes to `warning` and remain `editable-safe`.
- Dirty sections escalate the same issue codes back to `protected-view`.
