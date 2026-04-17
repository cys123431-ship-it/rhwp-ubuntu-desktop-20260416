# Wave 4 Fixture Roundtrip

- Date: 2026-04-17
- Command: `cargo run --quiet --bin rhwp -- compat-corpus compatibility-corpus/phase2-extended.tsv`
- Scope: `equation-basic.hwpx`, `ruby-basic.hwpx`, `hidden-comment.hwpx`

## Result

- All three fixtures reported `editable-safe`.
- All three fixtures preserved `preferredSaveFormat = hwpx`.
- All three fixtures completed `save-reparse` through `phase2-extended.tsv`.
- The only reported issue was `font-substitution` with `warning` severity.

## Notes

- These fixtures are tracked in `compatibility-corpus/phase2-extended.tsv`.
- Snapshot coverage is provided by `equation_basic_compatibility_report`, `ruby_basic_compatibility_report`, and `hidden_comment_compatibility_report`.
