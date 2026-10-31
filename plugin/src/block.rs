//! block：方框图（数据通路、存储层次、网络拓扑、I/O 结构）

use crate::parse::{err, fail, Attrs, Doc, Elem, Line, Res, Tok};
use crate::svg::{line_color, line_width, p, paint, Anchor, Canvas, Pen, INK, P};
use crate::timing::{clean, polygon};
use std::collections::HashMap;

const SHAPES: &[&str] = &[
    "box", "router", "switch", "hub", "host", "server", "cloud", "bus", "reg", "mem", "alu", "mux",
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Side {
    N,
    S,
    E,
    W,
}

impl Side {
    fn dir(self) -> P {
        match self {
            Side::N => p(0.0, -1.0),
            Side::S => p(0.0, 1.0),
            Side::E => p(1.0, 0.0),
            Side::W => p(-1.0, 0.0),
        }
    }
    fn parse(c: char) -> Option<Side> {
        match c {
            'n' => Some(Side::N),
            's' => Some(Side::S),
            'e' => Some(Side::E),
            'w' => Some(Side::W),
            _ => None,
        }
    }
}

struct Node {
    id: String,
    shape: String,
    text: String,
    c: P,
    hw: f64,
    hh: f64,
    attrs: Attrs,
    cells: usize,
    /// alu/mux 从宽边指向窄边的方向。
    dir: Side,
    outline: Vec<P>,
    /// 已有连线的边，用来给外置文字挑位置。
    used: Vec<Side>,
}

impl Node {
    fn l(&self) -> f64 {
        self.c.x - self.hw
    }
    fn r(&self) -> f64 {
        self.c.x + self.hw
    }
    fn t(&self) -> f64 {
        self.c.y - self.hh
    }
    fn b(&self) -> f64 {
        self.c.y + self.hh
    }

    /// 把 (u, v) ∈ [-1,1]² 映到图上：v 从宽边到窄边。
    fn taper(&self, u: f64, v: f64) -> P {
        self.c.add(match self.dir {
            Side::S => p(u * self.hw, v * self.hh),
            Side::N => p(u * self.hw, -v * self.hh),
            Side::E => p(v * self.hw, u * self.hh),
            Side::W => p(-v * self.hw, u * self.hh),
        })
    }

    fn make_outline(&mut self) {
        let ell = |c: P, a: f64, b: f64| -> Vec<P> {
            (0..48)
                .map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / 48.0;
                    c.add(p(a * t.cos(), b * t.sin()))
                })
                .collect()
        };
        self.outline = match self.shape.as_str() {
            "router" => ell(self.c, self.hw, self.hh),
            "cloud" => ell(self.c, self.hw * 1.1, self.hh * 1.1),
            "alu" => [
                (-1.0, -1.0),
                (-0.2, -1.0),
                (0.0, -0.5),
                (0.2, -1.0),
                (1.0, -1.0),
                (0.5, 1.0),
                (-0.5, 1.0),
            ]
            .iter()
            .map(|&(u, v)| self.taper(u, v))
            .collect(),
            "mux" => [(-1.0, -1.0), (1.0, -1.0), (0.55, 1.0), (-0.55, 1.0)]
                .iter()
                .map(|&(u, v)| self.taper(u, v))
                .collect(),
            _ => vec![
                p(self.l(), self.t()),
                p(self.r(), self.t()),
                p(self.r(), self.b()),
                p(self.l(), self.b()),
            ],
        };
    }

    /// 某条边上比例 f 处的接线点：从外接矩形向内投到实际轮廓。
    fn port(&self, side: Side, f: f64) -> P {
        let q = match side {
            Side::N => p(self.l() + 2.0 * self.hw * f, self.t()),
            Side::S => p(self.l() + 2.0 * self.hw * f, self.b()),
            Side::W => p(self.l(), self.t() + 2.0 * self.hh * f),
            Side::E => p(self.r(), self.t() + 2.0 * self.hh * f),
        };
        let inward = side.dir().mul(-1.0);
        let reach = 2.0 * (self.hw + self.hh) * 1.3;
        let start = q.sub(inward.mul(self.hh.max(self.hw)));
        let mut best: Option<f64> = None;
        let n = self.outline.len();
        for i in 0..n {
            let (a, b) = (self.outline[i], self.outline[(i + 1) % n]);
            let e = b.sub(a);
            let den = inward.x * e.y - inward.y * e.x;
            if den.abs() < 1e-9 {
                continue;
            }
            let d = a.sub(start);
            let t = (d.x * e.y - d.y * e.x) / den;
            let u = (d.x * inward.y - d.y * inward.x) / den;
            if (-1e-9..=1.0 + 1e-9).contains(&u) && t >= 0.0 && t <= reach {
                best = Some(best.map_or(t, |x: f64| x.min(t)));
            }
        }
        best.map_or(q, |t| start.add(inward.mul(t)))
    }

    /// 边界上 x（竖向接线）或 y（横向接线）对应的接线点。
    fn port_at(&self, side: Side, v: f64) -> P {
        let f = match side {
            Side::N | Side::S => (v - self.l()) / (2.0 * self.hw),
            _ => (v - self.t()) / (2.0 * self.hh),
        };
        self.port(side, f)
    }

    fn ray(&self, toward: P) -> P {
        let (a, b) = (self.c, toward);
        let d = b.sub(a);
        let mut best = 1.0_f64;
        let n = self.outline.len();
        for i in 0..n {
            let (u, v) = (self.outline[i], self.outline[(i + 1) % n]);
            let e = v.sub(u);
            let den = d.x * e.y - d.y * e.x;
            if den.abs() < 1e-9 {
                continue;
            }
            let w = u.sub(a);
            let t = (w.x * e.y - w.y * e.x) / den;
            let s = (w.x * d.y - w.y * d.x) / den;
            if (0.0..=1.0).contains(&s) && t > 0.0 {
                best = best.min(t);
            }
        }
        a.add(d.mul(best))
    }
}

#[derive(Clone)]
struct End {
    node: usize,
    port: Option<(Side, usize)>,
}

struct Edge {
    a: End,
    b: End,
    head_a: bool,
    head_b: bool,
    labels: Vec<String>,
    attrs: Attrs,
}

struct Free {
    at: P,
    text: String,
    attrs: Attrs,
}

fn parse_at(s: &str, no: usize) -> Res<(f64, f64)> {
    let bad = || fail(no, format!("coordinate `@{}` should be written as @col,row", s));
    let (a, b) = s.split_once(',').ok_or_else(bad)?;
    let num = |v: &str| v.trim().parse::<f64>().ok().filter(|x| x.is_finite());
    Ok((num(a).ok_or_else(bad)?, num(b).ok_or_else(bad)?))
}

/// 默认尺寸（半宽、半高）。
fn default_size(cv: &Canvas, shape: &str, text: &str, cells: usize, dir: Side) -> (f64, f64) {
    let (em, w) = (cv.em, cv.w(text));
    let along = |a: f64, b: f64| match dir {
        Side::E | Side::W => (b, a),
        _ => (a, b),
    };
    match shape {
        "router" => (em * 1.3, em * 1.3),
        "switch" => (em * 1.9, em * 0.8),
        "hub" => ((w / 2.0 + em * 0.6).max(em * 1.9), em * 0.7),
        "host" => ((w / 2.0 + em * 0.4).max(em * 1.1), em * 0.9),
        "server" => ((w / 2.0 + em * 0.5).max(em * 1.3), em * 1.4),
        "cloud" => (w / 2.0 + em * 1.0, em * 1.4),
        "reg" => ((w / 2.0 + em * 0.5).max(em * 1.2), em * 0.8),
        "mem" => (em * 1.8, cells as f64 * em * 0.5),
        "alu" => along((w / 2.0 + em * 1.2).max(em * 2.5), em * 1.5),
        "mux" => along(em * 2.0, (w / 2.0 + em * 0.6).max(em * 1.3)),
        _ => ((w / 2.0 + em * 0.7).max(em * 1.6), em * 1.0),
    }
}

fn parse_node(l: &Line, cv: &Canvas, gx: f64, gy: f64) -> Res<Node> {
    let t = &l.toks;
    let Some(Tok::E(id)) = t.first() else {
        return err(l.no, "expected a node id");
    };
    let mut shape = "box".to_string();
    let mut text = None;
    let mut at = None;
    let (mut w, mut h, mut cells, mut dir) = (None, None, 4usize, None);
    let mut attrs = id.attrs.clone();
    for (i, x) in t.iter().enumerate().skip(1) {
        match x {
            Tok::At(s) => at = Some(parse_at(s, l.no)?),
            Tok::Attrs(a) => attrs.merge(a.clone()),
            Tok::E(e) if e.quoted => {
                text = Some(e.text.clone());
                attrs.merge(e.attrs.clone());
            }
            Tok::E(e) => {
                attrs.merge(e.attrs.clone());
                if i == 1 && SHAPES.contains(&e.text.as_str()) {
                    shape = e.text.clone();
                } else if let Some((k, v)) = e.text.split_once('=') {
                    let num = v.parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0);
                    let bad = || fail(l.no, format!("`{}` must be a positive number", e.text));
                    match k {
                        "w" => w = Some(num.ok_or_else(bad)?),
                        "h" => h = Some(num.ok_or_else(bad)?),
                        "cells" => cells = num.ok_or_else(bad)? as usize,
                        "dir" => {
                            dir = Some(
                                v.chars()
                                    .next()
                                    .and_then(Side::parse)
                                    .ok_or_else(|| fail(l.no, "dir must be n, s, e or w"))?,
                            )
                        }
                        _ => {
                            return err(
                                l.no,
                                format!("unknown key `{}` (available: w h cells dir)", k),
                            )
                        }
                    }
                } else {
                    return err(
                        l.no,
                        format!(
                            "unexpected `{}`; a node is written `ID shape \"text\" @col,row w= h=` (shapes: {})",
                            e.text,
                            SHAPES.join(" ")
                        ),
                    );
                }
            }
            _ => return err(l.no, "unexpected token in a node line"),
        }
    }
    let Some((cx, cy)) = at else {
        return err(l.no, format!("node `{}` needs a coordinate `@col,row`", id.text));
    };
    let dir = dir.unwrap_or(if shape == "mux" { Side::E } else { Side::S });
    let text = text.unwrap_or_else(|| id.text.clone());
    let cells = cells.max(1);
    let (mut hw, mut hh) = default_size(cv, &shape, &text, cells, dir);
    if shape == "bus" {
        let thick = cv.em * 0.7;
        let vertical = h.is_some() && h.unwrap() * gy > w.unwrap_or(0.0) * gx;
        (hw, hh) = if vertical {
            (w.map_or(thick, |v| v * gx / 2.0), h.unwrap() * gy / 2.0)
        } else {
            (w.unwrap_or(4.0) * gx / 2.0, h.map_or(thick, |v| v * gy / 2.0))
        };
    } else {
        if let Some(v) = w {
            hw = v * gx / 2.0;
        }
        if let Some(v) = h {
            hh = v * gy / 2.0;
        }
        if shape == "router" {
            let r = if w.is_some() { hw } else { hh };
            (hw, hh) = (r, r);
        }
    }
    let mut n = Node {
        id: id.text.clone(),
        shape,
        text,
        c: p(cx * gx, cy * gy),
        hw,
        hh,
        attrs,
        cells,
        dir,
        outline: Vec::new(),
        used: Vec::new(),
    };
    n.make_outline();
    Ok(n)
}

fn parse_end(e: &Elem, nodes: &[Node], no: usize) -> Res<End> {
    if let Some(i) = nodes.iter().position(|n| n.id == e.text) {
        return Ok(End { node: i, port: None });
    }
    if let Some((id, port)) = e.text.rsplit_once('.') {
        if let Some(i) = nodes.iter().position(|n| n.id == id) {
            let mut cs = port.chars();
            let side = cs.next().and_then(Side::parse);
            let rest = cs.as_str();
            let k = if rest.is_empty() { Some(0) } else { rest.parse().ok() };
            return match (side, k) {
                (Some(s), Some(k)) => Ok(End {
                    node: i,
                    port: Some((s, k)),
                }),
                _ => err(
                    no,
                    format!("bad port `{}` (use n s e w, optionally numbered: e2)", port),
                ),
            };
        }
    }
    err(no, format!("unknown node `{}`", e.text))
}

fn parse_edge(l: &Line, nodes: &[Node]) -> Res<Edge> {
    let t = &l.toks;
    let bad = || fail(l.no, "an edge is written `A -> B \"label\" [attrs]`");
    let (Some(Tok::E(a)), Some(op), Some(Tok::E(b))) = (t.first(), t.get(1), t.get(2)) else {
        return Err(bad());
    };
    let (head_a, head_b) = match op {
        Tok::Dash => (false, false),
        Tok::Arrow => (false, true),
        Tok::DArrow => (true, true),
        _ => return Err(bad()),
    };
    let mut attrs = a.attrs.clone();
    attrs.merge(b.attrs.clone());
    let mut labels = Vec::new();
    for x in &t[3..] {
        match x {
            Tok::E(e) => {
                labels.push(e.text.clone());
                attrs.merge(e.attrs.clone());
            }
            Tok::Attrs(x) => attrs.merge(x.clone()),
            _ => return Err(bad()),
        }
    }
    if labels.len() > 2 {
        return err(l.no, "an edge takes at most two labels");
    }
    Ok(Edge {
        a: parse_end(a, nodes, l.no)?,
        b: parse_end(b, nodes, l.no)?,
        head_a,
        head_b,
        labels,
        attrs,
    })
}

/// 正交走线。a、b 为接线点，da、db 为两端朝外的方向。
fn route(a: P, da: P, b: P, db: P, stub: f64) -> Vec<P> {
    if da.y != 0.0 {
        let sw = |q: P| p(q.y, q.x);
        return route(sw(a), sw(da), sw(b), sw(db), stub)
            .into_iter()
            .map(sw)
            .collect();
    }
    let s = da.x;
    if db.x != 0.0 {
        if db.x == s {
            let x = if s > 0.0 { a.x.max(b.x) } else { a.x.min(b.x) } + s * stub;
            return vec![a, p(x, a.y), p(x, b.y), b];
        }
        if (b.x - a.x) * s > 0.0 {
            let mid = (a.x + b.x) / 2.0;
            return vec![a, p(mid, a.y), p(mid, b.y), b];
        }
        let ym = (a.y + b.y) / 2.0;
        let (a1, b1) = (a.x + s * stub, b.x - s * stub);
        return vec![a, p(a1, a.y), p(a1, ym), p(b1, ym), p(b1, b.y), b];
    }
    if (b.x - a.x) * s > 0.0 && (a.y - b.y) * db.y > 0.0 {
        return vec![a, p(b.x, a.y), b];
    }
    let (a1, b1) = (a.add(da.mul(stub)), b.add(db.mul(stub)));
    vec![a, a1, p(a1.x, b1.y), b1, b]
}

/// 一端已定时，另一端（node）该接在哪：对得上就直连，否则走 L 形或 Z 形。
fn auto_port(n: &Node, q: P, d: P, em: f64, stub: f64) -> (Side, P) {
    let m = em * 0.3;
    if d.y != 0.0 {
        let near = if d.y > 0.0 { Side::N } else { Side::S };
        let ahead = (n.c.y - q.y) * d.y > 0.0;
        if ahead && q.x > n.l() + m && q.x < n.r() - m {
            return (near, n.port_at(near, q.x));
        }
        if ahead {
            let s = if q.x < n.c.x { Side::W } else { Side::E };
            return (s, n.port(s, 0.5));
        }
        // 背离目标：先走一小段，再横着进最近的边。
        let y = q.y + d.y * stub;
        if y > n.t() + m && y < n.b() - m {
            let s = if q.x < n.c.x { Side::W } else { Side::E };
            return (s, n.port_at(s, y));
        }
        (near, n.port(near, 0.5))
    } else {
        let near = if d.x > 0.0 { Side::W } else { Side::E };
        let ahead = (n.c.x - q.x) * d.x > 0.0;
        if ahead && q.y > n.t() + m && q.y < n.b() - m {
            return (near, n.port_at(near, q.y));
        }
        if ahead {
            let s = if q.y < n.c.y { Side::N } else { Side::S };
            return (s, n.port(s, 0.5));
        }
        let x = q.x + d.x * stub;
        if x > n.l() + m && x < n.r() - m {
            let s = if q.y < n.c.y { Side::N } else { Side::S };
            return (s, n.port_at(s, x));
        }
        (near, n.port(near, 0.5))
    }
}

/// 两端都没指定：范围有重叠就直连，否则按主方向走 Z 形。
fn auto_both(a: &Node, b: &Node, em: f64) -> (Side, P, Side, P) {
    let m = em * 0.6;
    let pick = |lo: f64, hi: f64, ac: f64, aw: f64, bc: f64, bw: f64| {
        let v = if aw <= bw { ac } else { bc };
        v.clamp(lo.min(hi), lo.max(hi))
    };
    let (lo, hi) = (a.l().max(b.l()), a.r().min(b.r()));
    if hi - lo >= m && a.c.y != b.c.y {
        let x = pick(lo + m * 0.5, hi - m * 0.5, a.c.x, a.hw, b.c.x, b.hw);
        let (sa, sb) = if b.c.y > a.c.y {
            (Side::S, Side::N)
        } else {
            (Side::N, Side::S)
        };
        return (sa, a.port_at(sa, x), sb, b.port_at(sb, x));
    }
    let (lo, hi) = (a.t().max(b.t()), a.b().min(b.b()));
    if hi - lo >= m && a.c.x != b.c.x {
        let y = pick(lo + m * 0.5, hi - m * 0.5, a.c.y, a.hh, b.c.y, b.hh);
        let (sa, sb) = if b.c.x > a.c.x {
            (Side::E, Side::W)
        } else {
            (Side::W, Side::E)
        };
        return (sa, a.port_at(sa, y), sb, b.port_at(sb, y));
    }
    let dx = (b.c.x - a.c.x).abs() - a.hw - b.hw;
    let dy = (b.c.y - a.c.y).abs() - a.hh - b.hh;
    let (sa, sb) = if dy >= dx {
        if b.c.y > a.c.y {
            (Side::S, Side::N)
        } else {
            (Side::N, Side::S)
        }
    } else if b.c.x > a.c.x {
        (Side::E, Side::W)
    } else {
        (Side::W, Side::E)
    };
    (sa, a.port(sa, 0.5), sb, b.port(sb, 0.5))
}

fn side_of(d: P) -> Side {
    if d.x.abs() >= d.y.abs() {
        if d.x > 0.0 {
            Side::E
        } else {
            Side::W
        }
    } else if d.y > 0.0 {
        Side::S
    } else {
        Side::N
    }
}

/// 文字放线段旁边。at：0 靠 a 端，1 中点，2 靠 b 端。
fn seg_label(cv: &mut Canvas, a: P, b: P, s: &str, at: u8, side: P) {
    let em = cv.em;
    let (scale, pad) = (0.8, em * 0.4);
    let d = b.sub(a);
    if d.y.abs() < 1e-6 {
        let y = a.y + side.y.signum() * em * 0.6;
        let dirx = d.x.signum();
        let (x, anchor) = match at {
            1 => ((a.x + b.x) / 2.0, Anchor::Middle),
            0 => (
                a.x + dirx * pad,
                if dirx > 0.0 { Anchor::Start } else { Anchor::End },
            ),
            _ => (
                b.x - dirx * pad,
                if dirx > 0.0 { Anchor::End } else { Anchor::Start },
            ),
        };
        cv.text(p(x, y), s, anchor, scale, INK);
    } else if d.x.abs() < 1e-6 {
        let diry = d.y.signum();
        let y = match at {
            1 => (a.y + b.y) / 2.0,
            0 => a.y + diry * em * 0.7,
            _ => b.y - diry * em * 0.9,
        };
        let right = side.x >= 0.0;
        let x = a.x + if right { pad } else { -pad };
        let anchor = if right { Anchor::Start } else { Anchor::End };
        cv.text(p(x, y), s, anchor, scale, INK);
    } else {
        let (u, v) = match at {
            1 => (a, b),
            0 => (a, a.add(d.mul(0.4))),
            _ => (b.sub(d.mul(0.4)), b),
        };
        cv.edge_label(u, v, s, side.x >= 0.0);
    }
}

fn longest_seg(pts: &[P]) -> usize {
    (0..pts.len() - 1)
        .max_by(|&i, &j| {
            let (u, v) = (pts[i + 1].sub(pts[i]).len(), pts[j + 1].sub(pts[j]).len());
            u.partial_cmp(&v).unwrap()
        })
        .unwrap()
}

fn draw_labels(cv: &mut Canvas, pts: &[P], labels: &[String]) {
    let n = pts.len();
    let em = cv.em;
    // 默认放上方 / 右方；转弯处放到拐向的另一侧，避开折线本身。
    let dflt = p(1.0, -1.0);
    match labels {
        [] => {}
        [s] => {
            let i = longest_seg(pts);
            let side = if i + 2 < n {
                pts[i + 1].sub(pts[i + 2])
            } else if i > 0 {
                pts[i].sub(pts[i - 1])
            } else {
                dflt
            };
            seg_label(cv, pts[i], pts[i + 1], s, 1, nonzero(side, dflt));
        }
        [s1, s2] => {
            let side_a = if n > 2 { pts[1].sub(pts[2]) } else { dflt };
            let side_b = if n > 2 { pts[n - 2].sub(pts[n - 3]) } else { dflt };
            let mut side_b = nonzero(side_b, dflt);
            if n == 2 {
                let d = pts[1].sub(pts[0]);
                let room = if d.y.abs() < 1e-6 {
                    let need = cv.ws(s1, 0.8) + cv.ws(s2, 0.8) + em * 1.6;
                    need <= d.len()
                } else {
                    d.len() >= em * 3.2
                };
                if !room {
                    side_b = p(-1.0, 1.0);
                }
            }
            seg_label(cv, pts[0], pts[1], s1, 0, nonzero(side_a, dflt));
            seg_label(cv, pts[n - 2], pts[n - 1], s2, 2, side_b);
        }
        _ => unreachable!(),
    }
}

/// 只取方向中非零的分量，另一分量用默认值。
fn nonzero(d: P, dflt: P) -> P {
    p(
        if d.x.abs() > 1e-6 { d.x } else { dflt.x },
        if d.y.abs() > 1e-6 { d.y } else { dflt.y },
    )
}

fn cloud_path(n: &Node) -> (String, Vec<P>) {
    let k = 9;
    let v: Vec<P> = (0..k)
        .map(|i| {
            let t = std::f64::consts::TAU * (i as f64 + 0.5) / k as f64;
            n.c.add(p(n.hw * t.cos(), n.hh * t.sin()))
        })
        .collect();
    let mut d = format!("M{:.2} {:.2} ", v[0].x, v[0].y);
    let mut pts = Vec::new();
    for i in 0..k {
        let (a, b) = (v[i], v[(i + 1) % k]);
        let r = a.sub(b).len() * 0.58;
        d.push_str(&format!("A{:.2} {:.2} 0 0 1 {:.2} {:.2} ", r, r, b.x, b.y));
        // 包围盒取弧顶。
        let m = a.mid(b);
        pts.push(m.add(m.sub(n.c).norm().mul(r * 0.75)));
    }
    d.push('Z');
    (d, pts)
}

fn draw_node(cv: &mut Canvas, n: &Node) {
    let em = cv.em;
    let (fill, stroke, tc) = paint(n.attrs.color);
    let fill = fill.unwrap_or("#ffffff");
    let pen = Pen {
        color: stroke,
        width: cv.sw,
        dash: n.attrs.dashed,
    };
    let (x, y, w, h) = (n.l(), n.t(), n.hw * 2.0, n.hh * 2.0);
    let c = n.c;
    let thin = Pen {
        dash: false,
        ..pen
    };
    // 文字是否放在形状外面。
    let mut outside = false;
    let mut at = c;
    match n.shape.as_str() {
        "router" => {
            cv.circle(c, n.hw, Some(fill), Some(pen));
            let r = n.hw * 0.68;
            for (dx, dy) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                let k = std::f64::consts::FRAC_1_SQRT_2;
                let e = c.add(p(dx * k * r, dy * k * r));
                cv.arrow(c, e, thin);
            }
            outside = true;
        }
        "switch" => {
            cv.rect(x, y, w, h, 0.0, Some(fill), Some(pen));
            let a = n.hw * 0.62;
            let dy = n.hh * 0.42;
            cv.arrow(c.add(p(-a, -dy)), c.add(p(a, -dy)), thin);
            cv.arrow(c.add(p(a, dy)), c.add(p(-a, dy)), thin);
            outside = true;
        }
        "host" => cv.rect(x, y, w, h, 0.0, Some(fill), Some(pen)),
        "server" => {
            cv.rect(x, y, w, h, 0.0, Some(fill), Some(pen));
            for k in 1..=2 {
                let ly = y + k as f64 * em * 0.4;
                cv.line(p(x, ly), p(x + w, ly), thin);
            }
            at = p(c.x, y + em * 0.8 + (h - em * 0.8) / 2.0);
        }
        "cloud" => {
            let (d, pts) = cloud_path(n);
            cv.path(&d, Some(fill), Some(pen), &pts);
        }
        "bus" => {
            let f = paint(n.attrs.color).0.unwrap_or("#d4d4d4");
            cv.rect(x, y, w, h, 0.0, Some(f), Some(pen));
            outside = n.hh > n.hw || n.hh * 2.0 < em * 1.2;
        }
        "mem" => {
            cv.rect(x, y, w, h, 0.0, Some(fill), Some(pen));
            for k in 1..n.cells {
                let ly = y + h * k as f64 / n.cells as f64;
                cv.line(p(x, ly), p(x + w, ly), thin);
            }
            outside = n.cells > 1;
        }
        "alu" | "mux" => {
            polygon(cv, &n.outline, Some(fill), Some(pen));
            at = n.taper(0.0, 0.2);
            if n.shape == "mux" {
                at = n.taper(0.0, 0.0);
            }
        }
        _ => cv.rect(x, y, w, h, 0.0, Some(fill), Some(pen)),
    }
    if outside {
        let side = [Side::N, Side::S, Side::E, Side::W]
            .into_iter()
            .find(|s| !n.used.contains(s))
            .unwrap_or(Side::N);
        let pad = em * 0.7;
        let (q, anchor) = match side {
            Side::N => (p(c.x, n.t() - pad), Anchor::Middle),
            Side::S => (p(c.x, n.b() + pad), Anchor::Middle),
            Side::E => (p(n.r() + em * 0.4, c.y), Anchor::Start),
            Side::W => (p(n.l() - em * 0.4, c.y), Anchor::End),
        };
        cv.text(q, &n.text, anchor, 1.0, tc);
    } else {
        cv.label(at, &n.text, tc);
    }
}

pub fn block(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&[])?;
    let em = cv.em;
    let sp = cv.sp.max(0.5);
    let (gx, gy) = (em * 5.0 * sp, em * 3.0 * sp);
    let is_edge = |l: &Line| {
        l.toks
            .iter()
            .any(|t| matches!(t, Tok::Dash | Tok::Arrow | Tok::DArrow))
    };
    let is_text = |l: &Line| matches!(l.toks.first(), Some(Tok::E(e)) if e.is("text"));
    // 先收节点，边可以写在节点之前。
    let mut nodes: Vec<Node> = Vec::new();
    let mut frees: Vec<Free> = Vec::new();
    for l in &doc.lines {
        if is_edge(l) {
            continue;
        }
        if is_text(l) {
            let (mut at, mut text) = (None, None);
            let mut attrs = Attrs::default();
            for t in &l.toks[1..] {
                match t {
                    Tok::At(s) => at = Some(parse_at(s, l.no)?),
                    Tok::E(e) => {
                        text = Some(e.text.clone());
                        attrs.merge(e.attrs.clone());
                    }
                    Tok::Attrs(a) => attrs.merge(a.clone()),
                    _ => return err(l.no, "free text is written `text @col,row \"text\"`"),
                }
            }
            match (at, text) {
                (Some((x, y)), Some(text)) => frees.push(Free {
                    at: p(x * gx, y * gy),
                    text,
                    attrs,
                }),
                _ => return err(l.no, "free text is written `text @col,row \"text\"`"),
            }
            continue;
        }
        let n = parse_node(l, cv, gx, gy)?;
        if nodes.iter().any(|x| x.id == n.id) {
            return err(l.no, format!("duplicate node `{}`", n.id));
        }
        nodes.push(n);
    }
    if nodes.is_empty() && frees.is_empty() {
        return err(doc.head_line, "nothing to draw");
    }
    let edges: Vec<Edge> = doc
        .lines
        .iter()
        .filter(|l| is_edge(l))
        .map(|l| parse_edge(l, &nodes))
        .collect::<Res<_>>()?;

    // 同一条边上的编号端口按最大编号等分。
    let mut parts: HashMap<(usize, u8), usize> = HashMap::new();
    for e in &edges {
        for end in [&e.a, &e.b] {
            if let Some((s, k)) = end.port {
                let m = parts.entry((end.node, s as u8)).or_insert(0);
                *m = (*m).max(k);
            }
        }
    }
    let fixed = |end: &End| -> Option<(Side, P)> {
        let (s, k) = end.port?;
        let n = &nodes[end.node];
        let f = if k == 0 {
            0.5
        } else if n.shape == "mem" && matches!(s, Side::E | Side::W) && k <= n.cells {
            (k as f64 - 0.5) / n.cells as f64 // mem：第 k 格的中线
        } else {
            k as f64 / (parts[&(end.node, s as u8)] + 1) as f64
        };
        Some((s, n.port(s, f)))
    };

    let stub = em * 1.2;
    let mut paths: Vec<Vec<P>> = Vec::new();
    let mut used: Vec<(usize, Side)> = Vec::new();
    for e in &edges {
        let (na, nb) = (&nodes[e.a.node], &nodes[e.b.node]);
        let (fa, fb) = (fixed(&e.a), fixed(&e.b));
        if e.attrs.straight {
            let a = fa.map_or_else(|| na.ray(fb.map_or(nb.c, |x| x.1)), |x| x.1);
            let b = fb.map_or_else(|| nb.ray(a), |x| x.1);
            used.push((e.a.node, side_of(b.sub(a))));
            used.push((e.b.node, side_of(a.sub(b))));
            paths.push(vec![a, b]);
            continue;
        }
        let (sa, a, sb, b) = match (fa, fb) {
            (Some((sa, a)), Some((sb, b))) => (sa, a, sb, b),
            (Some((sa, a)), None) => {
                let (sb, b) = auto_port(nb, a, sa.dir(), em, stub);
                (sa, a, sb, b)
            }
            (None, Some((sb, b))) => {
                let (sa, a) = auto_port(na, b, sb.dir(), em, stub);
                (sa, a, sb, b)
            }
            (None, None) => auto_both(na, nb, em),
        };
        used.push((e.a.node, sa));
        used.push((e.b.node, sb));
        paths.push(clean(&route(a, sa.dir(), b, sb.dir(), stub)));
    }
    for (i, s) in used {
        nodes[i].used.push(s);
    }

    for (e, pts) in edges.iter().zip(&paths) {
        let pen = Pen {
            color: line_color(e.attrs.color),
            width: line_width(e.attrs.color, cv.sw * 1.1),
            dash: e.attrs.dashed,
        };
        match (e.head_a, e.head_b) {
            (false, false) => cv.polyline(pts, pen),
            (false, true) => cv.arrow_poly(pts, pen),
            _ => {
                let mut v = pts.clone();
                let n = v.len();
                let hl = cv.head_len() * 0.6;
                let d0 = v[0].sub(v[1]);
                let tip = v[0];
                v[0] = tip.sub(d0.norm().mul(hl));
                cv.head(tip, d0, pen.color);
                debug_assert!(n >= 2);
                cv.arrow_poly(&v, pen);
            }
        }
    }
    for n in &nodes {
        draw_node(cv, n);
    }
    for (e, pts) in edges.iter().zip(&paths) {
        draw_labels(cv, pts, &e.labels);
    }
    for f in &frees {
        cv.label(f.at, &f.text, paint(f.attrs.color).2);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_shapes() {
        let (e, w, s) = (p(1.0, 0.0), p(-1.0, 0.0), p(0.0, 1.0));
        let z = clean(&route(p(0.0, 0.0), e, p(10.0, 5.0), w, 2.0));
        assert_eq!(z.len(), 4);
        let straight = clean(&route(p(0.0, 0.0), e, p(10.0, 0.0), w, 2.0));
        assert_eq!(straight.len(), 2);
        let l = clean(&route(p(0.0, 0.0), s, p(10.0, 5.0), w, 2.0));
        assert_eq!(l, vec![p(0.0, 0.0), p(0.0, 5.0), p(10.0, 5.0)]);
    }
}
