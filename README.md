# TUIList

[![Crates.io](https://img.shields.io/crates/v/tuilist.svg)](https://crates.io/crates/tuilist)
[![CI](https://github.com/christianhelle/tuilist/actions/workflows/ci.yml/badge.svg)](https://github.com/christianhelle/tuilist/actions/workflows/ci.yml)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A terminal client for Microsoft To Do, with vim-style keys. Three panes show your
lists, tasks and details, an offline cache opens instantly, and sync runs in the
background.

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```
