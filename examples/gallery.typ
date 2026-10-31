// One picture with every kind. Regenerate with `make images`.
#import "@preview/dsviz:0.2.2": ds

#set page(width: 30cm, height: auto, margin: 8mm, fill: white)
#set text(size: 8pt)
#show raw.where(lang: "ds"): ds.with(size: 7.5pt, font: ("Helvetica Neue", "PingFang SC"))

#let cell(name, body, span: 1) = grid.cell(colspan: span, block(breakable: false, width: 100%, inset: (bottom: 6pt))[
  #text(8pt, font: "Helvetica Neue", weight: "bold", name)
  #v(-2pt)
  #align(center + horizon, body)
])

#grid(columns: (1fr,) * 4, column-gutter: 6mm, row-gutter: 5mm,
  cell[tree][```ds
forest
A
  B
    E
  C
D
  F
```],
  cell[bintree][```ds
bintree thread=in
A
  B
    _
    D
  C
```],
  cell[rbtree][```ds
rbtree
10
  5[red]
  20
    15[red]
    25[red]
```],
  cell[btree][```ds
btree
30 60
  10 20
  40 50
  70 80
```],
  cell[bplus][```ds
bplus
20 40
  5 20
  30 40
```],
  cell(span: 2)[list][```ds
list
L: 头[gray] -> a_1 -> a_2[^p] -> ^
x <-> y <-> z @1
```],
  cell[array][```ds
array index=0
第1趟: 3[red] 1[hl] 2[green] _ 5[blue,^low]
```],
  cell[stack][```ds
stack index=0 size=4
a b[hl] c[green,^top]
```],
  cell[ring][```ds
ring size=6
_ a[^front] b c _[^rear]
```],
  cell[matrix][```ds
matrix labels=A,B,C
0 1 1
1 0 .
1 . 0
```],
  cell[table][```ds
table head=row
j 1 2 3
next 0 1 1
```],
  cell[graph][```ds
graph layout=circle
A - B 5
B - C 2 [red]
C - D 4
D - A 7
A - C 3
```],
  cell[digraph][```ds
digraph dir=lr
A -> B
A -> C 3 [blue]
B -> D
C -> D [dashed]
```],
  cell(span: 2)[orthlist][```ds
orthlist
A -> B
A -> C
C -> A
B -> C
```],
  cell(span: 2)[multilist][```ds
multilist
A - B
A - C
B - C
```],
  cell(span: 2)[fields][```ds
fields ticks=on
虚拟地址: 页目录:10[blue] | 页表:10[hl] | 偏移:12[green]
```],
  cell[pipeline][```ds
pipeline cycles=7
I1: IF ID EX M WB
I2: _ IF ID stall EX M WB
I1.M -> I2.EX [red]
```],
  cell[gantt][```ds
gantt
P1: 0-3[blue] 7-9[blue]
P2: 3-7[hl]
```],
  cell(span: 2)[ladder][```ds
ladder actors=A,B
A -> B "SYN, seq=x"
note A "SYN-SENT"
B -> A "SYN+ACK"
A -x B "数据"
```],
  cell(span: 2)[wave][```ds
wave grid=on bits=1 0 1
CLK: c c c
曼彻斯特: 10 01 10
地址: =A1 . x
读: 1 0 1
```],
  cell(span: 2)[block][```ds
block
R1 router @0,0
R2 router @2,0
S1 switch @0,1
H1 host @1,1.6
N cloud "Internet" @3.6,0 w=1.2
R1 - R2 "E0" "E1"
R1 - S1 [green]
S1 - H1
R2 - N
```],
)
