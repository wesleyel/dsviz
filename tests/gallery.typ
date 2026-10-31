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

#pagebreak()
= 408 题图：位段 / 流水线 / 甘特图

#case[fields：虚拟地址 + 指令格式（默认标位宽）][```ds
fields
虚拟地址: 页目录号:10 | 页表索引:10[hl] | 页内偏移:12
OP:6 | Rs:5 | Rd:5 | imm:16
```]

#case[fields ticks=on：位序号，高位在左][```ds
fields ticks=on
虚拟地址: 页目录号:10 | 页表索引:10[hl] | 页内偏移:12
OP:6 | Rs:5 | Rd:5 | imm:16
```]

#case[fields：IEEE754，1 位字段走最小宽度；引号标题][```ds
fields ticks=on
"单精度 float": S:1 | 阶码:8[blue] | 尾数:23
"双精度 double": S:1 | 阶码:11[blue] | 尾数:52
```]

#case[pipeline：阻塞与转发][```ds
pipeline cycles=8
I1: IF ID EX M WB
I2: _ IF ID stall EX M WB
I1.M -> I2.EX
```]

#case[pipeline：同一缝隙多条箭头、跨行箭头、`*`、着色][```ds
pipeline
add: IF ID EX[hl] M WB
sub: _ IF ID EX[hl] M WB
and: _ _ IF ID EX[hl] M WB
or: _ _ _ IF ID * EX M WB
add.EX -> sub.EX
add.M -> and.EX [dashed]
sub.M -> and.EX
add.WB -> or.EX
```]

#case[gantt：基本写法][```ds
gantt
P1: 0-3 7-9
P2: 3-7[hl]
```]

#case[gantt：区间文字、小数时刻、单行][```ds
gantt
CPU: 0-3:P_1 3-4.5:P_2[blue] 4.5-10:"运行 P_3"[hl] 10-11:P_1
I/O: 3-5:"读盘" 8-11:"写盘"[gray]
```]

#case[gantt：密集时刻错行；axis=off][
  ```ds
  gantt
  P1: 0-1 1-2 2-3 97-100
  P2: 3-50[hl]
  ```
  #v(0.6em)
  ```ds
  gantt axis=off
  0-3:A 3-7:B[hl] 7-9:C
  ```
]

#pagebreak()
= 408 题图：时序图 / 波形

#case[ladder：TCP 三次握手 + 丢失][```ds
ladder actors=客户,服务器
客户 -> 服务器 "SYN, seq=x"
note 客户 "SYN-SENT"
note 服务器 "SYN-RCVD"
服务器 -> 客户 "SYN+ACK, seq=y, ack=x+1"
note 客户 "ESTABLISHED"
客户 -> 服务器 "ACK, seq=x+1, ack=y+1"
note 服务器 "ESTABLISHED"
gap
客户 -x 服务器 "数据"
gap
客户 => 服务器 "超时重传" [dashed]
```]

#case[ladder：交叉报文（\@行号）、slope=2、三角色][```ds
ladder actors=A,B,DNS slope=2
A -> B "seg1" @1
B -> A "ack0" @1
A -> DNS "查询" @4
note B @4 "等待"
DNS -> A "响应"[blue] @6
```]

#case[wave：规格样例（x 不定、总线延续、比特标注）][```ds
wave grid=on bits=1 0 1
CLK: c c c
曼彻斯特: 10 01 10
地址: =A1 . x
读: 1 0 1
```]

#case[wave：差分曼彻斯特、高阻、长总线文字、周期内多电平][```ds
wave bits=0 1 1 0
CLK: clk clk clk clk
差分曼彻斯特: 10 10 01 01
数据: z =D_0[hl] ="读 数据" z
片选: 1 0 . 1
混合: 0101 1x z0 =A
```]

#pagebreak()
= 408 题图：方框图

#case[block：规格样例][```ds
block
R1 router @0,0
R2 router @3,0
S1 switch @0,2
H1 host @-1,3
N cloud "Internet" @5,0 w=1.5
R1 - R2 "E0: 1.1.1.1" "E1: 1.1.1.2"
R1 - S1
S1 - H1
R2 - N
B bus "内部总线" @0,5 w=8
PC reg @1,4
PC.s - B
```]

#case[block：单总线数据通路（端口、多线接总线、alu、mem）][```ds
block
M mem "主存" @2.75,-2.6 cells=2 w=1.6
B bus "内部总线" @3.5,0 w=7.5
PC reg @0.5,-1
MAR reg @2,-1
MDR reg @3.5,-1
R0 reg @6.5,-1
Y reg @5.23,1
A alu "ALU" @5.5,2.4 w=1.6
Z reg @7,1.4
PC.s -> B
B -> MAR.s
MDR.s <-> B
R0.s <-> B
MAR.n -> M.s1 "地址"
MDR.n <-> M.s2 "数据"
B -> Y.n
Y.s -> A.n1 [straight]
B -> A.n2
A.e -> Z.s
Z.n -> B [dashed]
```]

#case[block：inode 索引（mem 分格端口）][```ds
block
I mem "inode" @0,1 cells=4 h=1.6
D1 box "数据块 1" @3,0
D2 box "数据块 2" @3,1
P box "一级索引块" @3,2
D3 box "数据块 3" @5.5,2
I.e1 -> D1.w
I.e2 -> D2.w
I.e4 -> P.w
P.e -> D3.w "指向"[blue]
text @0,2.3 "直接 + 间接"
```]

#case[block：存储层次 / I/O（mux、server、hub、纵向总线、直线、虚线）][```ds
block
C box "CPU" @0,0
Cache box "Cache" @0,1.5 [hl]
MM mem "主存" @0,3 cells=3
D server "磁盘" @3,3
VB bus "I/O 总线" @2,1.5 h=4
H hub "集线器" @4,0
X mux "MUX" @4,1.5
C <-> Cache
Cache <-> MM
MM - VB
D - VB
H.s -> X.n [dashed]
X.w -> VB "选通"
C.e -> H.w [straight]
```]
