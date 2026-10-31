# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.2] - 2026-10-31

### Fixed

- Three-digit keys in tree and graph nodes stay circular instead of turning into rectangles.

## [0.2.1] - 2026-10-31

### Changed

- New label engine shared by `graph`, `digraph`, `block`, `ladder`, tree edge labels and `^pointer` names. Labels are placed after everything is drawn: beside their edge when ownership is clear, on the edge over a translucent white mask otherwise, and with a leader line as a last resort.
- Crowded diagrams respace automatically before a leader line is used, so `spacing` is a minimum.
- `block` spreads edges that would share a port, so separate links no longer merge into one line.
- `^pointer` arrows lengthen to keep names apart, and `matrix` columns widen to fit their labels.

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
