# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Favorites: mark lists with `*` to keep them above the smart views, with the
  account named when two favorites share a name. Favorites are kept locally and
  restored with the selected one on the next start.
- `o` or `gx` opens a task's web link in the default browser: the first link from the
  tasks pane, or the one under the cursor in the details.

## [0.1.1] - 2026-11-15

### Added

- The terminal UI: lists, tasks and details panes, vim keys, dialogs, and a
  `--demo` showcase that runs without a Microsoft account.
- Editing tasks, steps and lists in the demo, with writes that revert and say
  why when the service refuses them.
- Searching tasks across accounts with `/`.
- Signing in and out, and listing the signed-in accounts, with `todo login`,
  `todo logout` and `todo accounts`.
- Browsing real accounts from an offline cache, with sync in the background,
  on `F5`, and from `todo sync`.
- Update checks at start and with `todo check-update`, and installing a release
  with `todo update` or `U`.

### Changed

- The lists pane is wider, so account addresses and list names are readable.

### Fixed

- The details pane's arrow, page and jump keys move between tasks.

[Unreleased]: https://github.com/christianhelle/tuilist/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/christianhelle/tuilist/releases/tag/v0.1.1
