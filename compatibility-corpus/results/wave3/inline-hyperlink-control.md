# Wave 3 Inline Hyperlink Control

- Date: 2026-04-17
- Test: `test_supported_inline_hyperlink_control_roundtrip`

## Result

- Simple `Control::Hyperlink` paragraphs with display text now serialize as native HWPX hyperlink fields.
- Reparsed output comes back as `FieldType::Hyperlink` with a reconstructed field range.

## Guardrails

- The conversion only applies when the paragraph has enough text to cover the hyperlink display text.
- Cases without a safe text span remain under the existing `hwpx-hyperlink` preserve-only policy.
