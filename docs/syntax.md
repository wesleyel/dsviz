# dsviz syntax reference

## The `ds` function

```typ
#ds(width: auto, font: auto, size: auto, spacing: 1.0, stroke: auto, palette: "gray", body)
```

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `body` | `str`, `content` | — | The DSL source: a string, a raw block, or content holding one raw block. |
| `size` | `auto`, `length` | `auto` | Label size. `auto` uses the current text size. |
| `font` | `auto`, `str`, `array` | `auto` | Label font. `auto` uses the current text font. |
| `width` | `auto`, `length`, `relative` | `auto` | Scale the whole diagram (labels and strokes included) to this width. The diagram is redrawn, not scaled as an image. |
| `spacing` | `float` | `1.0` | Node spacing factor. Does not change the text size. |
| `stroke` | `auto`, `length` | `auto` | Line width. `auto` is 8% of the text size. |
| `palette` | `str` | `"gray"` | `"gray"` for black-and-white printing, `"color"` for color fills and edges. |

When used through `show raw.where(lang: "ds"): ds`, the diagram inherits the
style of raw text, which is a monospace font at 0.8× size by default. To use
your body font instead, pass it explicitly:

```typ
#show raw.where(lang: "ds"): ds.with(font: "Libertinus Serif", size: 10pt)
```


## Syntax

The first line names the **kind** followed by optional `key=value` options.
The following lines describe the structure. `//` starts a comment and blank
lines are ignored. Errors report the line number, counted from the first line
of the block.

**Elements** are written as `label[attr, ...]`:

- Emphasis: `hl`, `red`, `green`, `blue`, `gray`. In the default `gray` palette these names select a gray fill, and on edges a heavier line (`hl`/`red` 2×, `green`/`blue` 1.5×). With `palette: "color"` they are real colors
- `dashed` draws a dashed outline or edge; `box` draws a rectangular node
- `^name` attaches a named pointer arrow, e.g. `^p`, `^top`, `^front`
- Quote labels with spaces: `"a b"`
- `a_1` and `a_{12}` render subscripts; `\_` is a literal underscore
- A lone `_` is an empty slot

## Kinds

| Kind | How to write it | Options |
|---|---|---|
| `tree`, `forest` | Indentation gives parent/child; several unindented lines make a forest. Prefix a line with `label:` to label the edge to its parent. | |
| `bintree` | Indented (a single child is the left child; use `_` as a placeholder), or one line in level order: `A B C _ D`. | `thread=in\|pre\|post`, `bits=on` (0/1 edge labels), `ext=on` (external nodes), `layout=tidy` (compact; default places nodes by in-order index) |
| `rbtree` | Like `bintree`; nodes are black by default, `[red]` makes them red. | `nil=on`, plus `thread`, `bits` |
| `btree`, `bplus` | Indented, one node per line, keys separated by spaces. With keys + 1 children the edges attach between keys. | btree: `ext=on` (failure nodes); bplus: `link=off` (no leaf chain) |
| `list` | `L: head[gray] -> a_1 -> a_2[^p] -> ^`. `<->` for doubly linked; a leading `^ <->` means a null prev; a trailing `@k` links back to node *k*. Several lines are aligned by column (adjacency lists, hash chaining). | |
| `array` | One array per line, optionally prefixed with `name:`. `_` is an empty cell. | `index=0\|1` |
| `stack` | Bottom to top, drawn vertically; several lines side by side. Two pointers make a shared stack. | `index=`, `size=` (pad with empty slots) |
| `ring` | Circular queue, slots in index order; `_[^rear]` puts a pointer on an empty slot. | `size=`, `index=off` |
| `matrix` | Bracketed matrix; `.` leaves a cell blank; colors mark regions. | `rows=`, `cols=`, `labels=` (comma separated) |
| `table` | Grid of cells. | `head=row\|col\|both` |
| `graph`, `digraph` | Edges `A - B 5`, `A -> B 5 [red]`, `A <-> B`; chains like `A - B - C`. Nodes `A[hl]`, `A @1,0` (if every node has coordinates they are used as-is; y points down). Straight edges that would cross another node are drawn as arcs. | `layout=circle\|layer`, `dir=tb\|lr` |
| `orthlist` | Orthogonal list of a digraph: write edges `A -> B`; an optional first line `A B C D` fixes the vertex order. | |
| `multilist` | Adjacency multilist of an undirected graph: write edges `A - B`. | |
| `fields` | Bit fields, one row per line with an optional title: `VA: dir:10 \| table:10[hl] \| offset:12`. Widths are proportional with a minimum that fits the text; rows are left-aligned on one scale. | `ticks=on` (bit positions at field edges, high bit left; default labels each field's width) |
| `pipeline` | `I1: IF ID EX M WB`, one instruction per line. `_` is an idle cycle, `stall` or `*` a stall. `I1.M -> I2.EX [attrs]` draws a forward from the first such cell of each row. | `cycles=N` (default: longest row) |
| `gantt` | `P1: 0-3 7-9`, intervals may carry attributes and text: `3-7[hl]`, `0-3:P1`, `0-3:"two words"`. Times may be decimals; the row name is optional. | `axis=off` |
| `ladder` | Sequence diagram. `A -> B "text"` (thin), `A => B "text"` (thick), `A -x B "text"` (lost: stops halfway at a ×); `[dashed]` applies to all. Each message starts on the row after the previous one, or at `@row` (from 1) for crossing messages. `note A "text"` annotates the lifeline, by default on the row of A's latest message end; `gap` skips a row. | `actors=A,B` (2 to 5, required), `slope=` (rows per arrow, default 1) |
| `wave` | `name: cycles…`, separated by spaces. A cycle is level characters sharing its width (`0` `1` `x` unknown, `z` high-Z, e.g. `10`), `c`/`clk` (one clock, high then low), or `=text` (bus segment, `="two words"`). `.` extends the previous cycle. | `grid=on`, `bits=1 0 1` (labels above each cycle; takes the rest of the header up to the next `key=`) |
| `block` | Node lines `ID [shape] ["text"] @col,row [w=] [h=] [attrs]`; `@` is the node **center**, 1 cell ≈ 5em × 3em, `w`/`h` are in cells, all nodes need coordinates. Shapes: `box` `router` `switch` `hub` `host` `server` `cloud` `bus` `reg` `mem` (`cells=4`) `alu` `mux` (`dir=n\|s\|e\|w` points from wide to narrow side; alu defaults to `s`, mux to `e`). Edges `A -> B "label" [attrs]`, `A - B`, `A <-> B`, orthogonal by default, `[straight]` for a direct line, no obstacle avoidance. Ports `A.e`, or `A.e2` for the *k*-th of the side's equal divisions (highest *k* used on that side + 1 parts; on `mem` e/w, the *k*-th cell). `A - B "near A" "near B"` labels both ends. `text @col,row "text"` places free text. | |
