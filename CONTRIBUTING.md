# Contributing to Forensica

Thanks for helping! A few rules keep the project trustworthy:

1. **Offline only.** Do not add code or dependencies that perform network I/O. Online features must be separate opt-in plugins (see `docs/online-plugins.md`).
2. **Never modify the original file.** Work on in-memory copies.
3. **Indicators, not verdicts.** Every finding needs a confidence, a plain-language explanation and its limitations. Never call an image "fake" or "real".
4. **Tests.** Every module ships unit tests with small fixtures, preferably synthesised in code. Third-party fixtures must have a compatible licence and be listed in `testdata/SOURCES.md`.
5. **Docs.** Each algorithm has a page in `docs/modules/` describing what it detects, how, failure modes and references.
6. **Code style.** `cargo fmt`, `cargo clippy -- -D warnings`, `flutter analyze` must be clean. No `unwrap()` in non-test Rust code. Errors via `thiserror`.
7. **Licences.** Dependencies must be AGPL-3.0-compatible (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, LGPL...). Run `cargo deny check licenses`.

## Workflow
- One issue per task, one branch per issue (`feat/<slug>`, `fix/<slug>`), Conventional Commits.
- Open a PR; squash-merge when CI is green.

## Building
See `README.md`.
