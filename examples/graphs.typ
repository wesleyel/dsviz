#import "@preview/dsviz:0.1.0": ds
#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(size: 10pt)
#show raw.where(lang: "ds"): ds

#grid(
  columns: 3,
  column-gutter: 24pt,
  row-gutter: 18pt,
  align: center + horizon,
  ```ds
  graph
  A - B 6
  B - C 5
  C - D 3 [red]
  D - E 4 [red]
  E - F 2 [red]
  F - A 1 [red]
  A - C 5
  B - E 3 [red]
  ```,
  ```ds
  digraph
  V1 -> V2 3
  V1 -> V3 2
  V2 -> V4 2
  V2 -> V5 3
  V3 -> V4 4
  V3 -> V6 3
  V4 -> V6 2
  V5 -> V6 1 [red]
  ```,
  ```ds
  digraph
  1 @0,0
  2 @1,0
  3 @0,1
  5 @1,1
  4 @0,2
  7 @1,2
  8 @2,2
  1 -> 2 5
  1 -> 3 6 [red]
  2 -> 5 3
  3 -> 5 6 [red]
  3 -> 4 3
  4 -> 5 3
  4 -> 7 1
  4 -> 8 4
  5 -> 7 2 [red]
  ```,
  ```ds
  matrix labels=A,B,C,D
  0 1 1[red] 0
  1 0 . 1
  1[red] . 0 1
  0 1 1 0
  ```,
  ```ds
  orthlist
  A B C D
  A -> B
  A -> C
  C -> A
  C -> D
  D -> A
  ```,
  ```ds
  multilist
  A B C D
  A - B
  A - D
  C - B
  C - D
  ```,
)
