//! graph / digraph / orthlist（十字链表）/ multilist（邻接多重表）

use crate::linear::null_mark;
use crate::parse::{err, fail, Attrs, Doc, Elem, Res, Tok};
use crate::svg::{line_color, p, paint, Anchor, Canvas, Pen, Shape, INK, MUTED, P};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EKind {
    Line,
    Arrow,
    Both,
}

pub struct Edge {
    pub u: usize,
    pub v: usize,
    pub kind: EKind,
    pub weight: Option<String>,
    pub attrs: Attrs,
    pub no: usize,
}

pub struct Graph {
    pub names: Vec<String>,
    pub attrs: Vec<Attrs>,
    pub at: Vec<Option<(f64, f64)>>,
    pub edges: Vec<Edge>,
}

impl Graph {
    fn node(&mut self, e: &Elem) -> usize {
        let i = match self.names.iter().position(|n| *n == e.text) {
            Some(i) => i,
            None => {
                self.names.push(e.text.clone());
                self.attrs.push(Attrs::default());
                self.at.push(None);
                self.names.len() - 1
            }
        };
        let a = &mut self.attrs[i];
        if e.attrs.color.is_some() {
            a.color = e.attrs.color;
        }
        a.dashed |= e.attrs.dashed;
        a.ptrs.extend(e.attrs.ptrs.iter().cloned());
        i
    }
}

fn parse_at(s: &str, no: usize) -> Res<(f64, f64)> {
    let bad = || fail(no, format!("coordinate `@{}` should be written as @x,y", s));
    let (a, b) = s.split_once(',').ok_or_else(bad)?;
    Ok((
        a.trim().parse().map_err(|_| bad())?,
        b.trim().parse().map_err(|_| bad())?,
    ))
}

pub fn parse_graph(doc: &Doc) -> Res<Graph> {
    let mut g = Graph {
        names: Vec::new(),
        attrs: Vec::new(),
        at: Vec::new(),
        edges: Vec::new(),
    };
    for l in &doc.lines {
        let t = &l.toks;
        let has_op = t
            .iter()
            .any(|x| matches!(x, Tok::Dash | Tok::Arrow | Tok::DArrow));
        let mut i = 0;
        let read_node = |g: &mut Graph, i: &mut usize| -> Res<usize> {
            match t.get(*i) {
                Some(Tok::E(e)) => {
                    let id = g.node(e);
                    *i += 1;
                    if let Some(Tok::At(s)) = t.get(*i) {
                        g.at[id] = Some(parse_at(s, l.no)?);
                        *i += 1;
                    }
                    Ok(id)
                }
                _ => err(l.no, "expected a node here"),
            }
        };
        if !has_op {
            while i < t.len() {
                read_node(&mut g, &mut i)?;
            }
            continue;
        }
        let mut chain = vec![read_node(&mut g, &mut i)?];
        let mut ops = Vec::new();
        while let Some(op) = t.get(i) {
            let k = match op {
                Tok::Dash => EKind::Line,
                Tok::Arrow => EKind::Arrow,
                Tok::DArrow => EKind::Both,
                _ => break,
            };
            i += 1;
            ops.push(k);
            chain.push(read_node(&mut g, &mut i)?);
        }
        let mut weight = None;
        let mut attrs = Attrs::default();
        if let Some(Tok::E(e)) = t.get(i) {
            weight = Some(e.text.clone());
            if e.attrs.color.is_some() || e.attrs.dashed {
                attrs = e.attrs.clone();
            }
            i += 1;
        }
        if let Some(Tok::Attrs(a)) = t.get(i) {
            attrs = a.clone();
            i += 1;
        }
        if i < t.len() {
            return err(
                l.no,
                "unexpected trailing content; an edge is written `A - B weight [attrs]`",
            );
        }
        for (k, op) in ops.iter().enumerate() {
            g.edges.push(Edge {
                u: chain[k],
                v: chain[k + 1],
                kind: *op,
                weight: weight.clone(),
                attrs: attrs.clone(),
                no: l.no,
            });
        }
    }
    if g.names.is_empty() {
        return err(doc.head_line, "the graph is empty");
    }
    Ok(g)
}

// ---------------- 布局 ----------------

fn circle_layout(n: usize, step: f64) -> Vec<P> {
    if n == 1 {
        return vec![p(0.0, 0.0)];
    }
    let tau = std::f64::consts::TAU;
    let r = (step * n as f64 / tau).max(step * 0.75);
    (0..n)
        .map(|i| {
            let a = -tau / 4.0 + tau * i as f64 / n as f64;
            p(a.cos() * r, a.sin() * r)
        })
        .collect()
}

/// 最长路径分层（先把 DFS 回边反向去环）。返回各结点层号。
pub fn layering(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut adj = vec![Vec::new(); n];
    for &(u, v) in edges {
        if u != v {
            adj[u].push(v);
        }
    }
    let mut color = vec![0u8; n];
    let mut dag: Vec<(usize, usize)> = Vec::new();
    fn dfs(u: usize, adj: &[Vec<usize>], color: &mut [u8], dag: &mut Vec<(usize, usize)>) {
        color[u] = 1;
        for &v in &adj[u] {
            match color[v] {
                0 => {
                    dag.push((u, v));
                    dfs(v, adj, color, dag);
                }
                1 => dag.push((v, u)),
                _ => dag.push((u, v)),
            }
        }
        color[u] = 2;
    }
    for s in 0..n {
        if color[s] == 0 {
            dfs(s, &adj, &mut color, &mut dag);
        }
    }
    let mut layer = vec![0usize; n];
    for _ in 0..n {
        let mut changed = false;
        for &(u, v) in &dag {
            if layer[v] < layer[u] + 1 {
                layer[v] = layer[u] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    layer
}

/// 重心法排层内次序。edges 只连相邻层。
pub fn ordering(layer: &[usize], edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let n = layer.len();
    let nl = layer.iter().max().map_or(0, |m| m + 1);
    let mut order: Vec<Vec<usize>> = vec![Vec::new(); nl];
    for v in 0..n {
        order[layer[v]].push(v);
    }
    let mut nb: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        nb[a].push(b);
        nb[b].push(a);
    }
    for it in 0..12 {
        let down = it % 2 == 0;
        let ls: Vec<usize> = if down {
            (1..nl).collect()
        } else {
            (0..nl.saturating_sub(1)).rev().collect()
        };
        for l in ls {
            let mut x = vec![0.0; n];
            for lv in &order {
                for (i, &v) in lv.iter().enumerate() {
                    x[v] = i as f64 - (lv.len() as f64 - 1.0) / 2.0;
                }
            }
            let from = if down { l.wrapping_sub(1) } else { l + 1 };
            let mut keyed: Vec<(f64, usize)> = order[l]
                .iter()
                .map(|&v| {
                    let ns: Vec<f64> = nb[v]
                        .iter()
                        .filter(|&&u| layer[u] == from)
                        .map(|&u| x[u])
                        .collect();
                    let k = if ns.is_empty() {
                        x[v]
                    } else {
                        ns.iter().sum::<f64>() / ns.len() as f64
                    };
                    (k, v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            order[l] = keyed.into_iter().map(|(_, v)| v).collect();
        }
    }
    order
}

/// 边 u→v 的直线若穿过其它结点，返回足以绕开它们的弯曲量（相对弦长），否则为 0。
fn detour(pos: &[P], shapes: &[Shape], u: usize, v: usize, em: f64) -> f64 {
    let (a, b) = (pos[u], pos[v]);
    let d = b.sub(a);
    let l2 = d.x * d.x + d.y * d.y;
    if l2 < 1e-9 {
        return 0.0;
    }
    let mut clear: f64 = 0.0;
    for (w, &c) in pos.iter().enumerate() {
        if w == u || w == v {
            continue;
        }
        let t = (c.sub(a).x * d.x + c.sub(a).y * d.y) / l2;
        if t <= 0.0 || t >= 1.0 {
            continue;
        }
        let r = shapes[w].half().0.max(shapes[w].half().1) + em * 0.3;
        let dist = c.sub(a.add(d.mul(t))).len();
        if dist < r {
            clear = clear.max(dist + r + em * 0.3);
        }
    }
    if clear == 0.0 {
        return 0.0;
    }
    // 两个控制点同向偏移 off 时，三次贝塞尔中点偏移 0.75·off
    let mag = (clear / (0.75 * l2.sqrt())).max(0.18);
    // 弯向其它结点总体更稀疏的一侧
    let crowd = |bend: f64| {
        let off = d.perp().mul(bend * 0.75);
        let mut s = 0.0;
        for t in [0.25, 0.5, 0.75] {
            let q = a.add(d.mul(t)).add(off);
            for (w, &c) in pos.iter().enumerate() {
                if w != u && w != v {
                    s += 1.0 / (q.sub(c).len().powi(2) + 1e-6);
                }
            }
        }
        s
    };
    if crowd(-mag) < crowd(mag) - 1e-9 {
        -mag
    } else {
        mag
    }
}

pub fn graph(doc: &Doc, cv: &mut Canvas, directed: bool) -> Res<()> {
    doc.check_opts(&["layout", "dir"])?;
    let g = parse_graph(doc)?;
    let n = g.names.len();
    let em = cv.em;
    let shapes: Vec<Shape> = g.names.iter().map(|s| cv.node_shape(s)).collect();
    let maxhw = shapes.iter().map(|s| s.half().0).fold(0.0, f64::max);
    let maxhh = shapes.iter().map(|s| s.half().1).fold(0.0, f64::max);
    let fixed = g.at.iter().filter(|a| a.is_some()).count();
    // 跨层边经过的虚拟结点位置
    let mut routes: Vec<Vec<P>> = vec![Vec::new(); g.edges.len()];
    let pos: Vec<P> = if fixed == n {
        let unit = em * 3.2 * cv.sp;
        g.at.iter()
            .map(|a| p(a.unwrap().0 * unit, a.unwrap().1 * unit))
            .collect()
    } else if fixed > 0 {
        return err(
            doc.head_line,
            "give coordinates `@x,y` to either all nodes or none",
        );
    } else {
        let layout = doc.opt_choice(
            "layout",
            &["circle", "layer"],
            if directed { "layer" } else { "circle" },
        )?;
        let dir = doc.opt_choice("dir", &["tb", "lr"], "tb")?;
        if layout == "circle" {
            circle_layout(n, (2.0 * maxhw + em * 1.8) * cv.sp)
        } else {
            let es: Vec<(usize, usize)> = g.edges.iter().map(|e| (e.u, e.v)).collect();
            let mut layer = layering(n, &es);
            // 跨多层的边拆成经过虚拟结点的链
            let mut short: Vec<(usize, usize)> = Vec::new();
            let mut chain: Vec<Vec<usize>> = vec![Vec::new(); es.len()];
            for (i, &(u, v)) in es.iter().enumerate() {
                let (lo, hi) = if layer[u] <= layer[v] { (u, v) } else { (v, u) };
                let mut prev = lo;
                for l in layer[lo] + 1..layer[hi] {
                    let d = layer.len();
                    layer.push(l);
                    short.push((prev, d));
                    chain[i].push(d);
                    prev = d;
                }
                if layer[lo] != layer[hi] {
                    short.push((prev, hi));
                }
                if lo != u {
                    chain[i].reverse();
                }
            }
            let order = ordering(&layer, &short);
            let mut all = vec![p(0.0, 0.0); layer.len()];
            for l in &order {
                for (i, &v) in l.iter().enumerate() {
                    let k = i as f64 - (l.len() as f64 - 1.0) / 2.0;
                    let d = layer[v] as f64;
                    all[v] = if dir == "tb" {
                        p(
                            k * (2.0 * maxhw + em * 1.8) * cv.sp,
                            d * (2.0 * maxhh + em * 2.0) * cv.sp,
                        )
                    } else {
                        p(
                            d * (2.0 * maxhw + em * 2.6) * cv.sp,
                            k * (2.0 * maxhh + em * 1.4) * cv.sp,
                        )
                    };
                }
            }
            for (i, c) in chain.iter().enumerate() {
                routes[i] = c.iter().map(|&d| all[d]).collect();
            }
            all.truncate(n);
            all
        }
    };
    let cen = pos
        .iter()
        .fold(p(0.0, 0.0), |a, b| a.add(*b))
        .mul(1.0 / n as f64);
    // 平行边计数
    for (ei, e) in g.edges.iter().enumerate() {
        let color = line_color(e.attrs.color);
        let pen = Pen {
            color,
            width: cv.sw,
            dash: e.attrs.dashed,
        };
        let (pu, pv) = (pos[e.u], pos[e.v]);
        let mut label_at: P;
        let mut side: P;
        if !routes[ei].is_empty() {
            let r = &routes[ei];
            let a = shapes[e.u].boundary(pu, r[0]);
            let b = shapes[e.v].boundary(pv, r[r.len() - 1]);
            let mut pts = vec![a];
            pts.extend(r.iter().copied());
            pts.push(b);
            cv.spline(&pts, pen, e.kind != EKind::Line, e.kind == EKind::Both);
            label_at = r[r.len() / 2];
            side = p(1.0, 0.0);
        } else if e.u == e.v {
            let (_, hh) = shapes[e.u].half();
            let top = p(pu.x, pu.y - hh);
            let a = shapes[e.u].boundary(pu, pu.add(p(-0.5, -1.0)));
            let b = shapes[e.u].boundary(pu, pu.add(p(0.5, -1.0)));
            let up = em * 2.0;
            cv.curve(
                a,
                p(a.x - em * 0.8, top.y - up),
                p(b.x + em * 0.8, top.y - up),
                b,
                pen,
                e.kind != EKind::Line,
            );
            label_at = p(pu.x, top.y - up * 0.75);
            side = p(0.0, -1.0);
        } else {
            // 同一对结点之间的第几条边
            let same: Vec<usize> = g
                .edges
                .iter()
                .enumerate()
                .filter(|(_, f)| (f.u == e.u && f.v == e.v) || (f.u == e.v && f.v == e.u))
                .map(|(i, _)| i)
                .collect();
            let k = same.iter().position(|&i| i == ei).unwrap();
            let total = same.len();
            let mut bend: f64;
            if total > 1 {
                // 方向相反的一对边都向各自左侧弯；同向的则分向两侧
                let pair_opposite = total == 2 && g.edges[same[0]].u != g.edges[same[1]].u;
                let rank = k as f64 - (total as f64 - 1.0) / 2.0;
                bend = if pair_opposite { 0.18 } else { rank * 0.36 };
                if g.edges[same[0]].u != e.u && !pair_opposite {
                    bend = -bend;
                }
            } else {
                // 直线会穿过其它结点时改走弧线
                bend = detour(&pos, &shapes, e.u, e.v, em);
            }
            if bend == 0.0 {
                let a = shapes[e.u].boundary(pu, pv);
                let b = shapes[e.v].boundary(pv, pu);
                match e.kind {
                    EKind::Line => cv.line(a, b, pen),
                    EKind::Arrow => cv.arrow(a, b, pen),
                    EKind::Both => {
                        cv.arrow(a, b, pen);
                        cv.head(a, a.sub(b), color);
                    }
                }
                let d = b.sub(a).norm();
                side = d.perp();
                let m = a.mid(b);
                if m.add(side).sub(cen).len() < m.sub(side).sub(cen).len() {
                    side = side.mul(-1.0);
                }
                label_at = m;
            } else {
                let d = pv.sub(pu);
                let off = d.perp().mul(bend);
                let ca = pu.add(d.mul(0.25)).add(off);
                let cb = pu.add(d.mul(0.75)).add(off);
                let a = shapes[e.u].boundary(pu, ca);
                let b = shapes[e.v].boundary(pv, cb);
                let dd = b.sub(a);
                let off = dd.perp().mul(bend);
                let c1 = a.add(dd.mul(0.25)).add(off);
                let c2 = a.add(dd.mul(0.75)).add(off);
                cv.curve(a, c1, c2, b, pen, e.kind != EKind::Line);
                if e.kind == EKind::Both {
                    cv.head(a, a.sub(c1), color);
                }
                label_at = crate::svg::bez(a, c1, c2, b, 0.5);
                side = off.norm();
            }
        }
        if let Some(w) = &e.weight {
            let tw = cv.ws(w, 0.85);
            let ext = side.x.abs() * tw / 2.0 + side.y.abs() * em * 0.45;
            label_at = label_at.add(side.mul(em * 0.3 + ext));
            cv.text(
                label_at,
                w,
                Anchor::Middle,
                0.85,
                if e.attrs.color.is_some() { color } else { INK },
            );
        }
    }
    for v in 0..n {
        cv.node(pos[v], shapes[v], &g.names[v], &g.attrs[v]);
        if !g.attrs[v].ptrs.is_empty() {
            let dir = p(-0.8, -1.0);
            cv.pointer(
                shapes[v].boundary(pos[v], pos[v].add(dir)),
                dir,
                &g.attrs[v].ptrs,
            );
        }
    }
    Ok(())
}

// ---------------- 十字链表 / 邻接多重表 ----------------

struct Box4 {
    x: f64,
    y: f64,
    cells: Vec<f64>,
}

impl Box4 {
    fn cell_x(&self, i: usize) -> f64 {
        self.x + self.cells[..i].iter().sum::<f64>() + self.cells[i] / 2.0
    }
    fn width(&self) -> f64 {
        self.cells.iter().sum()
    }
}

fn draw_box(
    cv: &mut Canvas,
    b: &Box4,
    h: f64,
    texts: &[(usize, &str)],
    fill: Option<&str>,
    pen: Pen,
) {
    cv.rect(b.x, b.y - h / 2.0, b.width(), h, 0.0, fill, Some(pen));
    let mut x = b.x;
    for c in &b.cells[..b.cells.len() - 1] {
        x += c;
        cv.line(p(x, b.y - h / 2.0), p(x, b.y + h / 2.0), pen);
    }
    for (i, t) in texts {
        cv.label(p(b.cell_x(*i), b.y), t, INK);
    }
}

/// 从指针域中心连到目标格子（上边或下边），同一行时从上方绕过去。
fn link(cv: &mut Canvas, a: P, tx: f64, ty: f64, h: f64, pen: Pen) {
    let em = cv.em;
    let (top, bot) = (ty - h / 2.0, ty + h / 2.0);
    if top > a.y + h * 0.6 {
        let k = (em * 1.2).max((top - a.y) * 0.4);
        cv.curve(a, p(a.x, a.y + k), p(tx, top - k), p(tx, top), pen, true);
    } else if bot < a.y - h * 0.6 {
        let k = (em * 1.2).max((a.y - bot) * 0.4);
        cv.curve(a, p(a.x, a.y - k), p(tx, bot + k), p(tx, bot), pen, true);
    } else {
        let k = em * 1.6;
        cv.curve(
            a,
            p(a.x, a.y - h / 2.0 - k),
            p(tx, top - k),
            p(tx, top),
            pen,
            true,
        );
    }
}

pub fn orthlist(doc: &Doc, cv: &mut Canvas, multi: bool) -> Res<()> {
    doc.check_opts(&[])?;
    let g = parse_graph(doc)?;
    for e in &g.edges {
        match (multi, e.kind) {
            (false, EKind::Arrow) | (true, EKind::Line) => {}
            (false, _) => {
                return err(
                    e.no,
                    "orthlist is for directed graphs; write edges as `A -> B`",
                )
            }
            (true, _) => {
                return err(
                    e.no,
                    "multilist is for undirected graphs; write edges as `A - B`",
                )
            }
        }
        if e.u == e.v {
            return err(e.no, "self-loops are not supported");
        }
    }
    let n = g.names.len();
    let em = cv.em;
    let h = em * 1.6;
    let pw = em * 1.0;
    let pen = cv.pen();
    let dw = g.names.iter().map(|s| cv.w(s)).fold(0.0, f64::max).max(em) + em * 0.8;
    let rowh = h + em * 2.4 * cv.sp;
    let iw = (0..n).map(|i| cv.w(&i.to_string())).fold(0.0, f64::max) + em * 0.8;
    let iw = iw.max(em * 1.4);
    // 顶点表
    let vcells = if multi {
        vec![dw, pw]
    } else {
        vec![dw, pw, pw]
    };
    let vboxes: Vec<Box4> = (0..n)
        .map(|i| Box4 {
            x: 0.0,
            y: i as f64 * rowh,
            cells: vcells.clone(),
        })
        .collect();
    // 边结点：所在行、列
    let ecells = if multi {
        vec![iw, pw, iw, pw]
    } else {
        vec![iw, iw, pw, pw]
    };
    let aw: f64 = ecells.iter().sum();
    let mut cols: Vec<usize> = g.edges.iter().map(|e| e.v).collect();
    cols.sort();
    cols.dedup();
    let x0 = vboxes[0].width() + em * 3.0 * cv.sp;
    let colw = aw + em * 1.8 * cv.sp;
    let eboxes: Vec<Box4> = g
        .edges
        .iter()
        .map(|e| {
            let c = cols.iter().position(|&c| c == e.v).unwrap();
            Box4 {
                x: x0 + c as f64 * colw,
                y: e.u as f64 * rowh,
                cells: ecells.clone(),
            }
        })
        .collect();
    for (i, e) in g.edges.iter().enumerate() {
        if g.edges[..i]
            .iter()
            .any(|f| (f.u, f.v) == (e.u, e.v) || (multi && (f.u, f.v) == (e.v, e.u)))
        {
            return err(e.no, "duplicate edge");
        }
    }
    // 画框
    for (i, b) in vboxes.iter().enumerate() {
        let (fill, _, _) = paint(g.attrs[i].color);
        draw_box(cv, b, h, &[(0, &g.names[i])], fill, pen);
        cv.text(p(-em * 0.4, b.y), &i.to_string(), Anchor::End, 0.75, MUTED);
    }
    for (b, e) in eboxes.iter().zip(&g.edges) {
        let (fill, _, _) = paint(e.attrs.color);
        let (us, vs) = (e.u.to_string(), e.v.to_string());
        let t: Vec<(usize, &str)> = if multi {
            vec![(0, &us), (2, &vs)]
        } else {
            vec![(0, &us), (1, &vs)]
        };
        draw_box(cv, b, h, &t, fill, pen);
    }
    if !multi {
        // 出边按列排、入边按行排
        for v in 0..n {
            let mut outs: Vec<usize> = (0..g.edges.len()).filter(|&i| g.edges[i].u == v).collect();
            outs.sort_by(|&a, &b| eboxes[a].x.partial_cmp(&eboxes[b].x).unwrap());
            let mut ins: Vec<usize> = (0..g.edges.len()).filter(|&i| g.edges[i].v == v).collect();
            ins.sort_by_key(|&i| g.edges[i].u);
            // firstout：域 2；firstin：域 1
            let vb = &vboxes[v];
            let fo = p(vb.cell_x(2), vb.y);
            match outs.first() {
                Some(&a) => cv.arrow(fo, p(eboxes[a].x, eboxes[a].y), pen),
                None => null_mark(cv, fo, pw, h),
            }
            let fi = p(vb.cell_x(1), vb.y);
            match ins.first() {
                Some(&a) => link(cv, fi, eboxes[a].cell_x(1), eboxes[a].y, h, pen),
                None => null_mark(cv, fi, pw, h),
            }
            // tlink（域 3）沿行，hlink（域 2）沿列
            for (k, &a) in outs.iter().enumerate() {
                let s = p(eboxes[a].cell_x(3), eboxes[a].y);
                match outs.get(k + 1) {
                    Some(&b) => cv.arrow(s, p(eboxes[b].x, eboxes[b].y), pen),
                    None => null_mark(cv, s, pw, h),
                }
            }
            for (k, &a) in ins.iter().enumerate() {
                let s = p(eboxes[a].cell_x(2), eboxes[a].y);
                match ins.get(k + 1) {
                    Some(&b) => cv.arrow(s, p(s.x, eboxes[b].y - h / 2.0), pen),
                    None => null_mark(cv, s, pw, h),
                }
            }
        }
    } else {
        // 每个顶点按书写顺序串起依附于它的边
        for v in 0..n {
            let list: Vec<usize> = (0..g.edges.len())
                .filter(|&i| g.edges[i].u == v || g.edges[i].v == v)
                .collect();
            let target = |i: usize| -> (f64, f64) {
                let c = if g.edges[i].u == v { 0 } else { 2 };
                (eboxes[i].cell_x(c), eboxes[i].y)
            };
            let vb = &vboxes[v];
            let fe = p(vb.cell_x(1), vb.y);
            match list.first() {
                Some(&a) => {
                    let (tx, ty) = target(a);
                    if (ty - fe.y).abs() < 1e-6 && eboxes[a].x > fe.x && tx == eboxes[a].cell_x(0) {
                        cv.arrow(fe, p(eboxes[a].x, ty), pen);
                    } else {
                        link(cv, fe, tx, ty, h, pen);
                    }
                }
                None => null_mark(cv, fe, pw, h),
            }
            for (k, &a) in list.iter().enumerate() {
                let field = if g.edges[a].u == v { 1 } else { 3 };
                let s = p(eboxes[a].cell_x(field), eboxes[a].y);
                match list.get(k + 1) {
                    Some(&b) => {
                        let (tx, ty) = target(b);
                        link(cv, s, tx, ty, h, pen);
                    }
                    None => null_mark(cv, s, pw, h),
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn chains_and_weights() {
        let d = parse("graph\nA - B - C 5 [red]\nD @1,2").unwrap();
        let g = parse_graph(&d).unwrap();
        assert_eq!(g.names, vec!["A", "B", "C", "D"]);
        assert_eq!(g.edges.len(), 2);
        assert_eq!(g.edges[1].weight.as_deref(), Some("5"));
        assert!(g.edges[1].attrs.color.is_some());
        assert_eq!(g.at[3], Some((1.0, 2.0)));
    }

    #[test]
    fn layering_dag() {
        // A->B->D, A->C->D
        assert_eq!(
            layering(4, &[(0, 1), (1, 3), (0, 2), (2, 3)]),
            vec![0, 1, 1, 2]
        );
        // 有环也能分层
        assert_eq!(layering(3, &[(0, 1), (1, 2), (2, 0)]), vec![0, 1, 2]);
        // 重心排序消除交叉：0->3, 1->2
        let o = ordering(&[0, 0, 1, 1], &[(0, 3), (1, 2)]);
        assert_eq!(o[1], vec![3, 2]);
    }

    #[test]
    fn detour_around_nodes() {
        let pos = [p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0), p(10.0, 10.0)];
        let shapes = [Shape::Circle(3.0); 4];
        // 0→2 的直线穿过结点 1，要弯；0→1 不穿过任何结点，不弯
        let b = detour(&pos, &shapes, 0, 2, 2.0);
        assert!(b.abs() * 0.75 * 20.0 > 3.0);
        assert_eq!(detour(&pos, &shapes, 0, 1, 2.0), 0.0);
        // 弯向远离结点 3 的一侧（上方，y 为负）
        assert!(p(20.0, 0.0).perp().mul(b).y < 0.0);
    }

    #[test]
    fn graph_errors() {
        assert!(parse_graph(&parse("graph\nA - B 5 6").unwrap()).is_err());
        assert!(parse_graph(&parse("graph\nA - @1,2").unwrap()).is_err());
    }
}
