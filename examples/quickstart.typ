#import "@preview/dsviz:0.1.0": ds
#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(size: 11pt)

// Every ```ds block becomes a diagram.
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
