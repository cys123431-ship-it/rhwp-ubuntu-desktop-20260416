# Wave 2 Windows Hancom Manual Gate

Wave 2 is not considered fully promoted until the documents below are opened, saved, and reopened in Windows Hancom Hangul without a critical regression.

## Scope

- Ubuntu package build and desktop E2E are automated in CI.
- This checklist is the final manual compatibility gate for the Wave 2 shape/textbox/picture-caption/group promotion set.

## Documents

- `samples/tac-img-02.hwpx`
- `compatibility-corpus/fixtures/basic-shape.hwpx`
- `compatibility-corpus/fixtures/textbox-in-shape.hwpx`
- `compatibility-corpus/fixtures/picture-caption.hwpx`
- `compatibility-corpus/fixtures/shape-group.hwpx`

## Procedure

1. Open the document in `rhwp` on Ubuntu.
2. Save the document without format conversion.
3. Transfer the saved output to Windows with Hancom Hangul installed.
4. Open the saved output in Hancom Hangul.
5. Save the document again in Hancom Hangul.
6. Reopen the Hancom-saved file in Hancom Hangul.
7. Reopen the same file in `rhwp`.

## Pass criteria

- The document opens in both applications.
- No fatal corruption or unsupported-format error is shown.
- Page count does not change unexpectedly.
- Shape geometry, textbox text, picture caption, and grouping survive the round trip.
- `rhwp` still reports the document as `editable-safe` after the round trip when no later-wave feature is introduced.

## Failure logging

Record the following for every failure:

- document path
- failing step number
- screenshot or exported PDF if helpful
- `compat-report --json <file>` output from `rhwp`
- whether the issue is layout-only, warning-only, or data-loss/corruption
