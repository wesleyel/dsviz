#import "../lib.typ": ds

#set page(width: 16cm, height: 24cm, margin: 1cm)
#set text(size: 10pt)
#show raw.where(lang: "ds"): ds

#let case(title, body) = block(breakable: false, below: 1.4em)[
  #text(weight: "bold", fill: gray.darken(30%))[#title] \
  #body
]

= dsviz gallery

#case[tree / forest][```ds
forest
A
  B
    E
    F
  C
  D
    G
H
  I
  "long label"
```]

#case[bintree thread=in][```ds
bintree thread=in
A
  B
    _
    D
      F
  C
    E
      _
      G
```]

#case[bintree thread=pre][```ds
bintree thread=pre
A
  B
    _
    D
      F
  C
    E
      _
      G
```]

#case[bintree, level order + bits=on (Huffman)][```ds
bintree bits=on
100 40 60[hl] a_1 b_2 c d_{12}
```]

#case[bintree ext=on (decision tree) + pointer][```ds
bintree ext=on
5
  2[^p]
    1
    3
  8
    6
    9
```]

#case[rbtree nil=on][```ds
rbtree nil=on
13
  8[red]
    1
    11
  17[red]
    15
    25
```]

#case[btree][```ds
btree
30
  10 20
  40 50[hl] 60
```]

#case[bplus][```ds
bplus
50 99
  15 50
    3 15
    20 50
  70 99
    60 70
    80 99
```]

#case[list][```ds
list
L: head[gray] -> a_1 -> a_2[^p] -> a_3 -> ^
C: head[gray] -> x -> y -> z @1
D: head[gray] <-> p <-> q @1
^ <-> u <-> v -> ^
```]

#case[adjacency list / separate chaining][```ds
list
0: V_0[gray] -> 1 -> 3 -> ^
1: V_1[gray] -> 0 -> ^
2: V_2[gray] -> ^
```]

#case[array][```ds
array index=0
init: 49 38 65 97 76 13 27
pass_1: 38[hl] 49 65 76 13 27 97[gray]
search: 5 13 19 21[^mid] 37 56 64[^high] _ _
```]

#case[stack (incl. shared stack)][```ds
stack index=0 size=5
S: a b c[^top]
shared: a b _ d[^top1] e
```]

#case[ring][```ds
ring size=8
_ _ a[^front] b c _[^rear] _ _
```]

#case[matrix][```ds
matrix labels=A,B,C,D
0 1 1[red] 0
1 0 . 1
1[red] . 0 1
0 1 1 0
```]

#case[matrix, triangular region][```ds
matrix
a_11[blue] . . .
a_21[blue] a_22[blue] . .
a_31[blue] a_32[blue] a_33[blue] .
a_41[blue] a_42[blue] a_43[blue] a_44[blue]
```]

#case[table (KMP next)][```ds
table head=col
j 1 2 3 4 5 6
pattern a b a a b c
next 0 1 1 2 2 3[hl]
```]

#case[graph (circle layout)][```ds
graph
A - B 6
A - C 1[red]
A - D 5
B - C 5
C - D 5
B - E 3
C - E 6
C - F 4
E - F 6
D - F 2 [red]
```]

#case[digraph (layered layout)][```ds
digraph
V1 -> V2 3
V1 -> V3 2
V2 -> V4 2
V2 -> V5 3
V3 -> V4 4
V3 -> V6 3
V4 -> V6 2
V5 -> V6 1 [red]
```]

#case[digraph dir=lr, bidirectional edge, self-loop][```ds
digraph dir=lr
A -> B
B -> A
B -> C 7
C -> C
A <-> C [blue]
```]

#case[graph with fixed coordinates][```ds
graph
A[hl] @0,0
B @1,0
C @1,1
D @0,1
A - B - C - D - A
A - C [dashed]
```]

#case[orthlist (orthogonal list)][```ds
orthlist
A B C D
A -> B
A -> C
C -> A
C -> D
D -> A
D -> B
D -> C
```]

#case[multilist (adjacency multilist)][```ds
multilist
A B C D E
A - B
A - D
C - B
C - D
E - B
C - E
```]

#case[edge labels + box + layout=tidy][```ds
bintree layout=tidy
25
  <25: 14
    <14: "(-∞, 14)"[box]
    >14: "(14, 25)"[box]
  >25: 42
    <42: "(25, 42)"[box]
    >42: "(42, +∞)"[box]
```]

#case[btree ext=on (failure nodes)][```ds
btree ext=on
1 2
  3 4
  5
  6
```]

#case[parameters: size / width / spacing][
  #ds(size: 8pt, spacing: 0.8)[```
  bintree
  A B C D E F G
  ```]
  #h(1em)
  #ds(width: 4cm)[```
  bintree
  A B C D E F G
  ```]
]
