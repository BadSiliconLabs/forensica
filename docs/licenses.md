# Dependency licences

Forensica is AGPL-3.0-or-later. All dependencies must be compatible.

## Rust
Checked in CI with `cargo deny check licenses bans sources` (`core/deny.toml`). Allowed: MIT, MIT-0, Apache-2.0 (incl. LLVM exception), BSD-2/3-Clause, Zlib, Unicode-3.0, CC0-1.0, BSL-1.0. The same check **bans HTTP client crates** (reqwest, ureq, hyper, curl, …) so network code cannot sneak in through a dependency.

## Dart / Flutter (direct dependencies)
| Package | Licence |
|---|---|
| flutter_riverpod / riverpod | MIT |
| go_router, path_provider, material_ui, intl, flutter_localizations | BSD-3-Clause |
| file_picker | MIT |
| desktop_drop | Apache-2.0 |
| pdf | Apache-2.0 |
| flutter_rust_bridge | MIT |

## Test data
See `testdata/SOURCES.md`.
