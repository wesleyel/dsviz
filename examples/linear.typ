#import "@preview/dsviz:0.1.0": ds
#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(size: 10pt)
#show raw.where(lang: "ds"): ds

#grid(
  columns: 2,
  column-gutter: 28pt,
  row-gutter: 18pt,
  align: left + horizon,
  ```ds
  list
  L: head[gray] -> a_1 -> a_2[^p] -> a_3 -> ^
  C: head[gray] -> x -> y -> z @1
  ^ <-> u <-> v -> ^
  ```,
  ```ds
  list
  0: V_0[gray] -> 1 -> 3 -> ^
  1: V_1[gray] -> 0 -> ^
  2: V_2[gray] -> ^
  ```,
  ```ds
  array index=0
  init: 49 38 65 97 76 13 27
  pass_1: 38[hl] 49 65 76 13 27 97[gray]
  ```,
  ```ds
  stack index=0 size=5
  a b c[^top]
  ```,
  ```ds
  ring size=8
  _ _ a[^front] b c _[^rear] _ _
  ```,
  ```ds
  table head=col
  j 1 2 3 4 5 6
  pattern a b a a b c
  next 0 1 1 2 2 3[hl]
  ```,
)
