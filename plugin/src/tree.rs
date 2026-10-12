//! tree / forest / bintree / rbtree / btree / bplus

use crate::parse::{err, Attrs, Color, Doc, Elem, Line, Res};
use crate::svg::{p, Anchor, Canvas, Pen, Shape, INK, MUTED, P};

// ---------------- 缩进解析 ----------------

pub struct INode {
    pub line: Line,
    pub kids: Vec<usize>,
}

/// 按缩进建立父子关系，返回结点表与根列表。
pub fn indent_tree(lines: &[Line]) -> Res<(Vec<INode>, Vec<usize>)> {
    let mut nodes: Vec<INode> = Vec::new();
    let mut roots = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut child_indent: Vec<Option<usize>> = Vec::new();
    let root_indent = lines.first().map(|l| l.indent).unwrap_or(0);
    for l in lines {
        while let Some(&t) = stack.last() {
            if nodes[t].line.indent >= l.indent {
                stack.pop();
            } else {
                break;
            }
        }
        let id = nodes.len();
        match stack.last() {
            None => {
                if l.indent != root_indent {
                    return err(l.no, "indentation does not match any line above");
                }
                roots.push(id);
            }
            Some(&par) => {
                match child_indent[par] {
                    Some(ci) if ci != l.indent => {
                        return err(l.no, "indentation is inconsistent with sibling nodes")
                    }
                    _ => child_indent[par] = Some(l.indent),
                }
                nodes[par].kids.push(id);
            }
        }
        nodes.push(INode {
            line: l.clone(),
            kids: Vec::new(),
        });
        child_indent.push(None);
        stack.push(id);
    }
    Ok((nodes, roots))
}

/// 一行一个结点，可在前面写 `边标签:` 标在连向双亲的边上。
fn single(l: &Line) -> Res<(Elem, Option<String>)> {
    let (lab, toks) = l.split_row_label();
    let es = Line {
        no: l.no,
        indent: l.indent,
        toks,
    }
    .elems()?;
    if es.len() != 1 {
        return err(l.no, "one node per line (quote labels that contain spaces)");
    }
    Ok((es.into_iter().next().unwrap(), lab))
}

// ---------------- 通用整齐布局 ----------------

/// 按轮廓整齐排布一般树。hw[i] 为结点半宽，返回各结点横坐标。
pub fn tidy(hw: &[f64], kids: &[Vec<usize>], roots: &[usize], gap: f64) -> Vec<f64> {
    let mut rel = vec![0.0; hw.len()];
    let (pos, _) = row(roots, hw, kids, gap, &mut rel);
    let mut x = vec![0.0; hw.len()];
    let mut st: Vec<(usize, f64)> = pos;
    while let Some((v, xv)) = st.pop() {
        x[v] = xv;
        for &k in &kids[v] {
            st.push((k, xv + rel[k]));
        }
    }
    x
}

type Contour = Vec<(f64, f64)>;

/// 排一棵子树，记录孩子相对双亲的偏移，返回以根为原点的轮廓。
fn lay(v: usize, hw: &[f64], kids: &[Vec<usize>], gap: f64, rel: &mut [f64]) -> Contour {
    let mut c = vec![(-hw[v], hw[v])];
    if kids[v].is_empty() {
        return c;
    }
    let (pos, sub) = row(&kids[v], hw, kids, gap, rel);
    let mid = (pos.first().unwrap().1 + pos.last().unwrap().1) / 2.0;
    for (k, o) in pos {
        rel[k] = o - mid;
    }
    c.extend(sub.into_iter().map(|(l, r)| (l - mid, r - mid)));
    c
}

/// 从左到右并排若干子树，返回 (子树根, 横坐标) 与合并后的轮廓。
fn row(
    list: &[usize],
    hw: &[f64],
    kids: &[Vec<usize>],
    gap: f64,
    rel: &mut [f64],
) -> (Vec<(usize, f64)>, Contour) {
    let mut acc: Contour = Vec::new();
    let mut pos = Vec::new();
    for &k in list {
        let c = lay(k, hw, kids, gap, rel);
        let mut off: f64 = 0.0;
        if !acc.is_empty() {
            off = f64::NEG_INFINITY;
            for d in 0..acc.len().min(c.len()) {
                off = off.max(acc[d].1 - c[d].0 + gap);
            }
        }
        for (d, &(l, r)) in c.iter().enumerate() {
            if d < acc.len() {
                acc[d] = (acc[d].0.min(l + off), acc[d].1.max(r + off));
            } else {
                acc.push((l + off, r + off));
            }
        }
        pos.push((k, off));
    }
    (pos, acc)
}

fn depths(kids: &[Vec<usize>], roots: &[usize]) -> Vec<usize> {
    let mut d = vec![0; kids.len()];
    let mut st: Vec<usize> = roots.to_vec();
    while let Some(v) = st.pop() {
        for &k in &kids[v] {
            d[k] = d[v] + 1;
            st.push(k);
        }
    }
    d
}

// ---------------- tree / forest ----------------

pub fn tree(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&[])?;
    if doc.lines.is_empty() {
        return err(doc.head_line, "the tree is empty");
    }
    let (nodes, roots) = indent_tree(&doc.lines)?;
    let parsed: Vec<(Elem, Option<String>)> =
        nodes.iter().map(|n| single(&n.line)).collect::<Res<_>>()?;
    let (elems, labs): (Vec<Elem>, Vec<Option<String>>) = parsed.into_iter().unzip();
    let shapes: Vec<Shape> = elems.iter().map(|e| cv.elem_shape(e)).collect();
    let hw: Vec<f64> = shapes.iter().map(|s| s.half().0).collect();
    let kids: Vec<Vec<usize>> = nodes.iter().map(|n| n.kids.clone()).collect();
    let xs = tidy(&hw, &kids, &roots, cv.em * 0.9 * cv.sp);
    let ds = depths(&kids, &roots);
    let lvl = cv.em * 2.6 * cv.sp;
    let pos: Vec<P> = (0..nodes.len())
        .map(|i| p(xs[i], ds[i] as f64 * lvl))
        .collect();
    let mut segs = Vec::new();
    for v in 0..nodes.len() {
        for &k in &kids[v] {
            let a = shapes[v].boundary(pos[v], pos[k]);
            let b = shapes[k].boundary(pos[k], pos[v]);
            segs.push((a, b));
            cv.line(
                a,
                b,
                Pen {
                    color: INK,
                    width: cv.sw,
                    dash: elems[k].attrs.dashed,
                },
            );
            if let Some(l) = &labs[k] {
                cv.edge_label(a, b, l, pos[k].x >= pos[v].x);
            }
        }
    }
    for v in 0..nodes.len() {
        cv.node(pos[v], shapes[v], elems[v].label(), &elems[v].attrs);
        tree_ptr(cv, pos[v], shapes[v], &elems[v].attrs, &pos, &segs);
    }
    Ok(())
}

/// 结点指针：在左上、右上、左、右中挑离其它结点和树边最远的方向。
fn tree_ptr(cv: &mut Canvas, c: P, s: Shape, a: &Attrs, all: &[P], segs: &[(P, P)]) {
    if a.ptrs.is_empty() {
        return;
    }
    let len = cv.em * 1.6;
    let clearance = |dir: P| {
        let tip = s.boundary(c, c.add(dir));
        let start = tip.add(dir.norm().mul(len));
        let mut best = f64::INFINITY;
        for q in [start, tip.mid(start)] {
            for &o in all {
                if o.sub(c).len() > 1e-6 {
                    best = best.min(q.sub(o).len());
                }
            }
            for &(u, v) in segs {
                best = best.min(seg_dist(q, u, v));
            }
        }
        best
    };
    let cands = [p(-0.8, -1.0), p(0.8, -1.0), p(-1.0, 0.0), p(1.0, 0.0)];
    let mut dir = cands[0];
    let mut score = clearance(dir);
    for &d in &cands[1..] {
        let sc = clearance(d);
        // 首选左上；其它方向要明显更空才换
        if sc > score + cv.em * 0.5 {
            dir = d;
            score = sc;
        }
    }
    let tip = s.boundary(c, c.add(dir));
    cv.pointer(tip, dir, &a.ptrs);
}

fn seg_dist(q: P, u: P, v: P) -> f64 {
    let d = v.sub(u);
    let l2 = d.x * d.x + d.y * d.y;
    let t = if l2 < 1e-9 {
        0.0
    } else {
        ((q.x - u.x) * d.x + (q.y - u.y) * d.y) / l2
    };
    q.sub(u.add(d.mul(t.clamp(0.0, 1.0)))).len()
}

// ---------------- bintree / rbtree ----------------

struct BNode {
    e: Elem,
    lab: Option<String>,
    l: Option<usize>,
    r: Option<usize>,
}

struct BTree {
    nodes: Vec<BNode>,
    root: Option<usize>,
}

fn parse_bintree(doc: &Doc) -> Res<BTree> {
    let mut t = BTree {
        nodes: Vec::new(),
        root: None,
    };
    if doc.lines.is_empty() {
        return Ok(t);
    }
    // 一行多个元素：顺序存储写法。
    if doc.lines.len() == 1 && doc.lines[0].toks.len() > 1 {
        let l = &doc.lines[0];
        let es = l.elems()?;
        let mut id: Vec<Option<usize>> = vec![None; es.len() + 1];
        for (i, e) in es.iter().enumerate() {
            let k = i + 1;
            if e.is_blank() {
                continue;
            }
            if k > 1 && id[k / 2].is_none() {
                return err(
                    l.no,
                    format!(
                        "`{1}` at position {0} has no parent (position {2} is empty)",
                        k,
                        e.text,
                        k / 2
                    ),
                );
            }
            let n = t.nodes.len();
            t.nodes.push(BNode {
                e: e.clone(),
                lab: None,
                l: None,
                r: None,
            });
            id[k] = Some(n);
            if k == 1 {
                t.root = Some(n);
            } else if k % 2 == 0 {
                t.nodes[id[k / 2].unwrap()].l = Some(n);
            } else {
                t.nodes[id[k / 2].unwrap()].r = Some(n);
            }
        }
        return Ok(t);
    }
    let (inodes, roots) = indent_tree(&doc.lines)?;
    if roots.len() != 1 {
        return err(
            inodes[roots[1]].line.no,
            "a binary tree has exactly one root (only one unindented line)",
        );
    }
    let mut map: Vec<Option<usize>> = vec![None; inodes.len()];
    for (i, n) in inodes.iter().enumerate() {
        let (e, lab) = single(&n.line)?;
        if e.is_blank() {
            if !n.kids.is_empty() {
                return err(n.line.no, "an empty slot `_` cannot have children");
            }
            continue;
        }
        if n.kids.len() > 2 {
            return err(
                inodes[n.kids[2]].line.no,
                "a binary tree node has at most two children",
            );
        }
        map[i] = Some(t.nodes.len());
        t.nodes.push(BNode {
            e,
            lab,
            l: None,
            r: None,
        });
    }
    for (i, n) in inodes.iter().enumerate() {
        if let Some(me) = map[i] {
            t.nodes[me].l = n.kids.first().and_then(|&k| map[k]);
            t.nodes[me].r = n.kids.get(1).and_then(|&k| map[k]);
        }
    }
    t.root = map[roots[0]];
    Ok(t)
}

#[derive(Clone, Copy, PartialEq)]
enum Slot {
    Real(usize),
    Ext(usize, bool), // 双亲, 是否右侧
}

fn order(t: &BTree, v: usize, how: &str, out: &mut Vec<usize>) {
    let n = &t.nodes[v];
    if how == "pre" {
        out.push(v);
    }
    if let Some(l) = n.l {
        order(t, l, how, out);
    }
    if how == "in" {
        out.push(v);
    }
    if let Some(r) = n.r {
        order(t, r, how, out);
    }
    if how == "post" {
        out.push(v);
    }
}

fn inorder_slots(t: &BTree, v: usize, ext: bool, out: &mut Vec<Slot>) {
    let n = &t.nodes[v];
    match n.l {
        Some(l) => inorder_slots(t, l, ext, out),
        None if ext => out.push(Slot::Ext(v, false)),
        None => {}
    }
    out.push(Slot::Real(v));
    match n.r {
        Some(r) => inorder_slots(t, r, ext, out),
        None if ext => out.push(Slot::Ext(v, true)),
        None => {}
    }
}

pub fn bintree(doc: &Doc, cv: &mut Canvas, rb: bool) -> Res<()> {
    if rb {
        doc.check_opts(&["nil", "thread", "bits", "layout"])?;
    } else {
        doc.check_opts(&["thread", "bits", "ext", "layout"])?;
    }
    let layout = doc.opt_choice("layout", &["inorder", "tidy"], "inorder")?;
    let thread = doc.opt_choice("thread", &["in", "pre", "post", "off"], "off")?;
    let bits = doc.opt_bool("bits", false)?;
    let ext = if rb {
        doc.opt_bool("nil", false)?
    } else {
        doc.opt_bool("ext", false)?
    };
    let t = parse_bintree(doc)?;
    let Some(root) = t.root else {
        return err(doc.first_line(), "the binary tree is empty");
    };
    let em = cv.em;
    let shapes: Vec<Shape> = t.nodes.iter().map(|n| cv.elem_shape(&n.e)).collect();
    let nil_w = cv.ws("NIL", 0.6) / 2.0 + em * 0.25;
    let ext_shape = if rb {
        Shape::Rect {
            hw: nil_w,
            hh: em * 0.4,
            rx: 0.0,
        }
    } else {
        Shape::Rect {
            hw: em * 0.42,
            hh: em * 0.42,
            rx: 0.0,
        }
    };
    let mut slots = Vec::new();
    inorder_slots(&t, root, ext, &mut slots);
    let maxhw = shapes.iter().map(|s| s.half().0).fold(0.0, f64::max);
    let step = (maxhw + em * 0.4) * cv.sp;
    let lvl = em * 2.5 * cv.sp;
    // 深度
    let mut depth = vec![0usize; t.nodes.len()];
    let mut st = vec![root];
    while let Some(v) = st.pop() {
        for k in [t.nodes[v].l, t.nodes[v].r].into_iter().flatten() {
            depth[k] = depth[v] + 1;
            st.push(k);
        }
    }
    let mut pos = vec![p(0.0, 0.0); t.nodes.len()];
    let mut ext_pos: Vec<(usize, bool, P)> = Vec::new();
    if layout == "inorder" {
        // 横坐标取中序序号
        for (i, s) in slots.iter().enumerate() {
            let x = i as f64 * step;
            match *s {
                Slot::Real(v) => pos[v] = p(x, depth[v] as f64 * lvl),
                Slot::Ext(par, right) => {
                    ext_pos.push((par, right, p(x, (depth[par] + 1) as f64 * lvl)))
                }
            }
        }
    } else {
        // 紧凑排布：缺失的孩子用不画出的占位结点保住左右位置
        let n = t.nodes.len();
        let mut hw: Vec<f64> = shapes.iter().map(|s| s.half().0).collect();
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut exts: Vec<(usize, usize, bool)> = Vec::new();
        for v in 0..n {
            let (l, r) = (t.nodes[v].l, t.nodes[v].r);
            if l.is_none() && r.is_none() && !ext {
                continue;
            }
            for (c, right) in [(l, false), (r, true)] {
                match c {
                    Some(c) => kids[v].push(c),
                    None => {
                        let id = hw.len();
                        hw.push(if ext { ext_shape.half().0 } else { em * 0.5 });
                        kids.push(Vec::new());
                        kids[v].push(id);
                        if ext {
                            exts.push((id, v, right));
                        }
                    }
                }
            }
        }
        let xs = tidy(&hw, &kids, &[root], em * 0.8 * cv.sp);
        for v in 0..n {
            pos[v] = p(xs[v], depth[v] as f64 * lvl);
        }
        for (id, par, right) in exts {
            ext_pos.push((par, right, p(xs[id], (depth[par] + 1) as f64 * lvl)));
        }
    }
    // 边
    let edge_pen = |cv: &Canvas, e: &Elem| Pen {
        color: INK,
        width: cv.sw,
        dash: e.attrs.dashed,
    };
    let mut edges: Vec<(P, P, bool)> = Vec::new();
    for v in 0..t.nodes.len() {
        for (k, right) in [(t.nodes[v].l, false), (t.nodes[v].r, true)] {
            if let Some(k) = k {
                let a = shapes[v].boundary(pos[v], pos[k]);
                let b = shapes[k].boundary(pos[k], pos[v]);
                let pen = edge_pen(cv, &t.nodes[k].e);
                cv.line(a, b, pen);
                if let Some(l) = &t.nodes[k].lab {
                    cv.edge_label(a, b, l, right);
                }
                edges.push((a, b, right));
            }
        }
    }
    for &(par, right, q) in &ext_pos {
        let a = shapes[par].boundary(pos[par], q);
        let b = ext_shape.boundary(q, pos[par]);
        cv.line(a, b, cv.pen());
        edges.push((a, b, right));
    }
    if bits {
        for &(a, b, right) in &edges {
            let m = a.mid(b);
            let d = b.sub(a).norm();
            let mut n = d.perp();
            if (n.x < 0.0) == right {
                n = n.mul(-1.0);
            }
            cv.text(
                m.add(n.mul(em * 0.55)),
                if right { "1" } else { "0" },
                Anchor::Middle,
                0.85,
                INK,
            );
        }
    }
    // 线索
    if thread != "off" {
        let mut seq = Vec::new();
        order(&t, root, thread, &mut seq);
        let mut idx = vec![0; t.nodes.len()];
        for (i, &v) in seq.iter().enumerate() {
            idx[v] = i;
        }
        let obstacles: Vec<P> = pos.clone();
        let pen = Pen {
            color: MUTED,
            width: cv.sw,
            dash: true,
        };
        for v in 0..t.nodes.len() {
            for right in [false, true] {
                let has = if right {
                    t.nodes[v].r.is_some()
                } else {
                    t.nodes[v].l.is_some()
                };
                if has {
                    continue;
                }
                let target = if right {
                    seq.get(idx[v] + 1).copied()
                } else {
                    idx[v].checked_sub(1).map(|i| seq[i])
                };
                match target {
                    Some(u) => {
                        let a = shapes[v].boundary(pos[v], pos[u]);
                        let b = shapes[u].boundary(pos[u], pos[v]);
                        let bend = pick_bend(a, b, 0.22, &obstacles, &[pos[v], pos[u]]);
                        cv.bend_arrow(a, b, bend, pen, true);
                    }
                    None => {
                        let sgn = if right { 1.0 } else { -1.0 };
                        let (hw, _) = shapes[v].half();
                        let nw = cv.ws("NULL", 0.8);
                        let tc = pos[v].add(p(sgn * (hw + em * 1.6 + nw / 2.0), em * 0.9));
                        cv.text(tc, "NULL", Anchor::Middle, 0.8, MUTED);
                        let tip = tc.add(p(-sgn * (nw / 2.0 + em * 0.2), 0.0));
                        let a = shapes[v].boundary(pos[v], tip);
                        cv.arrow(a, tip, pen);
                    }
                }
            }
        }
    }
    // 结点
    let segs: Vec<(P, P)> = edges.iter().map(|&(a, b, _)| (a, b)).collect();
    for v in 0..t.nodes.len() {
        let e = &t.nodes[v].e;
        if rb && e.attrs.color.map_or(true, |c| c == Color::Red) {
            let red = e.attrs.color == Some(Color::Red);
            let fill = if red { "#c0392b" } else { INK };
            let stroke = Pen {
                color: fill,
                width: cv.sw,
                dash: e.attrs.dashed,
            };
            cv.shape(pos[v], shapes[v], Some(fill), Some(stroke));
            cv.label(pos[v], e.label(), "#ffffff");
        } else {
            cv.node(pos[v], shapes[v], e.label(), &e.attrs);
        }
        tree_ptr(cv, pos[v], shapes[v], &e.attrs, &pos, &segs);
    }
    for &(_, _, q) in &ext_pos {
        if rb {
            cv.shape(
                q,
                ext_shape,
                Some(INK),
                Some(Pen {
                    color: INK,
                    width: cv.sw,
                    dash: false,
                }),
            );
            cv.text(q, "NIL", Anchor::Middle, 0.6, "#ffffff");
        } else {
            cv.shape(q, ext_shape, None, Some(cv.pen()));
        }
    }
    Ok(())
}

/// 选择弧线弯曲方向，使曲线离其它结点更远。
pub fn pick_bend(a: P, b: P, mag: f64, obstacles: &[P], skip: &[P]) -> f64 {
    let score = |bend: f64| {
        let d = b.sub(a);
        let off = d.perp().mul(bend * 0.75);
        let mut best = f64::INFINITY;
        for t in [0.25, 0.5, 0.75] {
            let q = a.add(d.mul(t)).add(off);
            for o in obstacles {
                if skip.iter().any(|s| s.sub(*o).len() < 1e-6) {
                    continue;
                }
                best = best.min(q.sub(*o).len());
            }
        }
        best
    };
    if score(-mag) > score(mag) + 1e-6 {
        -mag
    } else {
        mag
    }
}

// ---------------- btree / bplus ----------------

pub fn btree(doc: &Doc, cv: &mut Canvas, plus: bool) -> Res<()> {
    if plus {
        doc.check_opts(&["link"])?;
    } else {
        doc.check_opts(&["ext"])?;
    }
    let link = plus && doc.opt_bool("link", true)?;
    let ext = !plus && doc.opt_bool("ext", false)?;
    if doc.lines.is_empty() {
        return err(doc.head_line, "the tree is empty");
    }
    let (nodes, roots) = indent_tree(&doc.lines)?;
    let em = cv.em;
    let keys: Vec<Vec<Elem>> = nodes.iter().map(|n| n.line.elems()).collect::<Res<_>>()?;
    let cells: Vec<Vec<f64>> = keys
        .iter()
        .map(|ks| {
            ks.iter()
                .map(|k| (cv.w(k.label()) + em * 0.8).max(em * 1.6))
                .collect()
        })
        .collect();
    let hw: Vec<f64> = cells.iter().map(|c| c.iter().sum::<f64>() / 2.0).collect();
    let kids: Vec<Vec<usize>> = nodes.iter().map(|n| n.kids.clone()).collect();
    for (i, n) in nodes.iter().enumerate() {
        if n.kids.len() > keys[i].len() + 1 {
            return err(
                nodes[n.kids[keys[i].len() + 1]].line.no,
                "more children than keys + 1",
            );
        }
    }
    let xs = tidy(&hw, &kids, &roots, em * 1.0 * cv.sp);
    let ds = depths(&kids, &roots);
    let h = em * 1.7;
    let lvl = em * 3.2 * cv.sp;
    let pos: Vec<P> = (0..nodes.len())
        .map(|i| p(xs[i], ds[i] as f64 * lvl))
        .collect();
    // 边
    for v in 0..nodes.len() {
        let n = kids[v].len();
        let left = pos[v].x - hw[v];
        for (j, &k) in kids[v].iter().enumerate() {
            let x = if n == keys[v].len() + 1 {
                left + cells[v][..j].iter().sum::<f64>()
            } else {
                left + cells[v][..j].iter().sum::<f64>() + cells[v][j] / 2.0
            };
            let a = p(x, pos[v].y + h / 2.0);
            let b = p(pos[k].x, pos[k].y - h / 2.0);
            cv.line(
                a,
                b,
                Pen {
                    color: INK,
                    width: cv.sw,
                    dash: keys[k].iter().any(|e| e.attrs.dashed),
                },
            );
        }
    }
    // 失败结点：叶结点每个指针位置向下画箭头指向黑色小方块
    if ext {
        for v in 0..nodes.len() {
            if !kids[v].is_empty() {
                continue;
            }
            let mut x = pos[v].x - hw[v];
            let y0 = pos[v].y + h / 2.0;
            for j in 0..=cells[v].len() {
                let y1 = y0 + em * 1.3;
                cv.arrow(p(x, y0), p(x, y1), cv.pen());
                cv.rect(x - em * 0.22, y1, em * 0.44, em * 0.8, 0.0, Some(INK), None);
                if j < cells[v].len() {
                    x += cells[v][j];
                }
            }
        }
    }
    // 叶链
    if link {
        let mut leaves = Vec::new();
        fn dfs(v: usize, kids: &[Vec<usize>], out: &mut Vec<usize>) {
            if kids[v].is_empty() {
                out.push(v);
            }
            for &k in &kids[v] {
                dfs(k, kids, out);
            }
        }
        for &r in &roots {
            dfs(r, &kids, &mut leaves);
        }
        for w in leaves.windows(2) {
            let a = p(pos[w[0]].x + hw[w[0]], pos[w[0]].y);
            let b = p(pos[w[1]].x - hw[w[1]], pos[w[1]].y);
            if (a.y - b.y).abs() < 1e-6 {
                cv.arrow(a, b, cv.pen());
            } else {
                let mx = (a.x + b.x) / 2.0;
                cv.arrow_poly(&[a, p(mx, a.y), p(mx, b.y), b], cv.pen());
            }
        }
    }
    // 结点
    for v in 0..nodes.len() {
        let mut x = pos[v].x - hw[v];
        let y = pos[v].y - h / 2.0;
        for (k, cw) in keys[v].iter().zip(&cells[v]) {
            let (fill, _, tc) = crate::svg::paint(k.attrs.color);
            if fill.is_some() {
                cv.rect(x, y, *cw, h, 0.0, fill, None);
            }
            cv.label(p(x + cw / 2.0, pos[v].y), k.label(), tc);
            if !k.attrs.ptrs.is_empty() {
                cv.pointer(p(x + cw / 2.0, y), p(0.0, -1.0), &k.attrs.ptrs);
            }
            x += cw;
        }
        let dash = keys[v].iter().all(|e| e.attrs.dashed) && !keys[v].is_empty();
        let pen = Pen {
            color: INK,
            width: cv.sw,
            dash,
        };
        cv.rect(pos[v].x - hw[v], y, hw[v] * 2.0, h, 0.0, None, Some(pen));
        let mut x = pos[v].x - hw[v];
        for cw in &cells[v][..cells[v].len().saturating_sub(1)] {
            x += cw;
            cv.line(p(x, y), p(x, y + h), pen);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn indent_structure() {
        let d = parse("tree\nA\n  B\n    D\n  C\nE").unwrap();
        let (n, r) = indent_tree(&d.lines).unwrap();
        assert_eq!(r, vec![0, 4]);
        assert_eq!(n[0].kids, vec![1, 3]);
        assert_eq!(n[1].kids, vec![2]);
    }

    #[test]
    fn bad_indent() {
        let d = parse("tree\nA\n    B\n  C").unwrap();
        assert!(indent_tree(&d.lines).err().unwrap().contains("line 4"));
    }

    #[test]
    fn sequential_bintree() {
        let d = parse("bintree\nA B C _ D").unwrap();
        let t = parse_bintree(&d).unwrap();
        assert_eq!(t.nodes.len(), 4);
        let b = t.nodes[0].l.unwrap();
        assert_eq!(t.nodes[b].e.text, "B");
        assert!(t.nodes[b].l.is_none());
        assert_eq!(t.nodes[t.nodes[b].r.unwrap()].e.text, "D");
        assert!(parse_bintree(&parse("bintree\nA _ C D").unwrap()).is_err());
    }

    #[test]
    fn indent_bintree_placeholder() {
        let d = parse("bintree\nA\n  _\n  C\n    D").unwrap();
        let t = parse_bintree(&d).unwrap();
        let a = &t.nodes[t.root.unwrap()];
        assert!(a.l.is_none());
        assert_eq!(t.nodes[a.r.unwrap()].e.text, "C");
        assert_eq!(t.nodes[t.nodes[a.r.unwrap()].l.unwrap()].e.text, "D");
    }

    #[test]
    fn edge_labels_and_box() {
        let d = parse("bintree\nA\n  <A: B[box]\n  >A: C").unwrap();
        let t = parse_bintree(&d).unwrap();
        let b = &t.nodes[t.nodes[t.root.unwrap()].l.unwrap()];
        assert_eq!(b.lab.as_deref(), Some("<A"));
        assert!(b.e.attrs.boxed);
    }

    #[test]
    fn tidy_no_overlap() {
        // A(B(D,E),C(F,G))
        let kids = vec![
            vec![1, 2],
            vec![3, 4],
            vec![5, 6],
            vec![],
            vec![],
            vec![],
            vec![],
        ];
        let hw = vec![1.0; 7];
        let x = tidy(&hw, &kids, &[0], 1.0);
        assert!(x[4] + 1.0 + 1.0 <= x[5] + 1e-9);
        assert!((x[0] - (x[1] + x[2]) / 2.0).abs() < 1e-9);
    }
}
