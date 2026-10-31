# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-10-31

### Added

- Kinds: `fields`, `pipeline`, `gantt`, `ladder`, `wave`, `block`.
- `[straight]` edge attribute (used by `block`).
- `palette` parameter: `"color"` (default) or `"gray"` for black-and-white printing.

### Changed

- Unified style across kinds: pure black ink and square corners. With `palette: "gray"`, color names select gray levels, and on edges a heavier line.
- README split into English and Chinese, with the syntax reference moved to `docs/syntax.md`.

## [0.1.0] - 2026-10-13

### Added

- `ds` function and `raw` show-rule integration.
- Kinds: `tree`, `forest`, `bintree`, `rbtree`, `btree`, `bplus`, `list`,
  `array`, `stack`, `ring`, `matrix`, `table`, `graph`, `digraph`,
  `orthlist`, `multilist`.
- Labels are typeset and measured by Typst, so any font lines up.
- Straight graph edges that would cross another node are routed as arcs.
