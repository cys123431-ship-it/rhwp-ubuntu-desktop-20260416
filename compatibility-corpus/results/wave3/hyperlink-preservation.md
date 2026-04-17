# Wave 3 Hyperlink Preservation

- Date: 2026-04-17
- Scope: HWPX hyperlink controls while native write is still pending

## Covered issue code

- `hwpx-hyperlink`

## Regression anchors

- Snapshot: `preserved_hyperlink_compatibility_report`
- DocumentCore test: `compatibility_report_marks_dirty_hyperlink_as_protected`

## Policy

- Clean snapshot-backed hyperlink controls are downgraded to `warning` and stay `editable-safe`.
- Dirty sections with hyperlink controls escalate back to `protected-view`.
