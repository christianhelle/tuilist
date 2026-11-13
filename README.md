# TUIList

[![Crates.io](https://img.shields.io/crates/v/tuilist.svg)](https://crates.io/crates/tuilist)
[![CI](https://github.com/christianhelle/tuilist/actions/workflows/ci.yml/badge.svg)](https://github.com/christianhelle/tuilist/actions/workflows/ci.yml)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A terminal client for Microsoft To Do, with vim-style keys. Three panes show your
lists, tasks and details, an offline cache opens instantly, and sync runs in the
background.

## Try it without signing in

```bash
cargo run -- --demo
```

The demo has two accounts and sample lists, so you can look around before
connecting a real Microsoft account.

## The screen

```
 Smart        Tasks                          Details
 Important    Today                          Title
 Planned      [ ] Buy milk                   Due
 All          [x] Send the status report     Notes and steps
 Lists
 ada@outlook.com
  Tasks
  Groceries
```

Move between panes with `Tab`, move with `j`/`k`, and press `?` for every key.

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```
