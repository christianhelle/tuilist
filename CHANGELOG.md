# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/christianhelle/tuilist/commits/main
