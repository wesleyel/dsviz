#import "@preview/dsviz:0.1.0": ds
#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(size: 10pt)
#show raw.where(lang: "ds"): ds

#grid(
  columns: 3,
  column-gutter: 24pt,
  row-gutter: 16pt,
  align: center + horizon,
  ```ds
  bintree bits=on
  100 40 60[hl] a_1 b_2 c d_12
  ```,
  ```ds
  rbtree nil=on
  13
    8[red]
      1
      11
    17[red]
      15
      25
  ```,
  ```ds
  bintree layout=tidy
  25
    <25: 14
      <14: "(-∞, 14)"[box]
      >14: "(14, 25)"[box]
    >25: 42[^p]
  ```,
  ```ds
  forest
  A
    B
    C
    D
  E
    F
  ```,
  grid.cell(colspan: 2)[```ds
  bplus
  50 99
    15 50
      3 15
      20 50
    70 99
      60 70
      80 99
  ```],
)
