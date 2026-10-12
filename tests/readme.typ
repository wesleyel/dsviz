// Snippets from README.md; keep in sync.
#import "@preview/dsviz:0.1.0": ds
#set page(width: 12cm, height: auto, margin: 1cm)

#show raw.where(lang: "ds"): ds

```ds
bintree thread=in
A
  B
    _
    D
  C
    E
```

#ds(width: 5cm, spacing: 1.2)[```
digraph
A -> B 3
B -> C 4
A -> C 9 [red]
```]

#ds("array index=0\n3 1[hl] 4 1 5")

#[
  #show raw.where(lang: "ds"): ds.with(font: "Libertinus Serif", size: 10pt)
  ```ds
  list
  // comments are ignored
  L: 头[gray] -> a_1 -> a_{12}[^p] -> "a b" -> \_x -> ^
  ```
]
