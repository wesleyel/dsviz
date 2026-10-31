// The same diagrams in both palettes. Regenerate with `make images`.
#import "@preview/dsviz:0.2.2": ds

#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(size: 10pt, font: ("Helvetica Neue", "PingFang SC"))

#let rb = "rbtree\n10\n  5[red]\n  20\n    15[red]\n    25[red]"
#let dg = "digraph dir=lr\nA -> B 2 [red]\nA -> C 3 [blue]\nB -> D 4\nC -> D 1 [green]"
#let arr = "array index=0\n3 1[hl] 2[red] 4[green] 5[blue]"

#grid(columns: 4, column-gutter: 16pt, row-gutter: 14pt, align: center + horizon,
  strong[color], ds(rb), ds(dg), ds(arr),
  strong[gray], ds(rb, palette: "gray"), ds(dg, palette: "gray"), ds(arr, palette: "gray"),
)
