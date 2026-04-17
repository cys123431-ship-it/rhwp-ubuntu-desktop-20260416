# Wave 3 Hyperlink Field Roundtrip

- Date: 2026-04-17
- Fixture: `compatibility-corpus/fixtures/hyperlink-field.hwpx`

## Result

- `sourceFormat = hwpx`
- `preferredSaveFormat = hwpx`
- `editMode = editable-safe`
- `roundtrip = save-reparse`

## Notes

- This fixture uses native HWPX `fieldBegin` / `fieldEnd` markup with `type="HYPERLINK"`.
- It is promoted into `compatibility-corpus/phase1-supported.tsv`.
