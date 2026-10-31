//! 统一的文字避让：所有图种画完线和结点后，在这里放「可移动」的文字。
//!
//! 边标按三档选位，保证标签归属明确且不压住任何东西：
//! 1. 贴在本边旁：不碰任何线、结点、文字，且明显离本边最近；
//! 2. 压在本边上：白底断开本边，归属由位置本身保证；
//! 3. 引线：挪到附近空白处，用细线连回本边。

use crate::svg::{p, Anchor, Canvas, Pen, Rect, MUTED, P};

/// 候选位置（沿边的比例），靠前的优先。
pub const MID_TS: [f64; 9] = [0.5, 0.42, 0.58, 0.34, 0.66, 0.26, 0.74, 0.18, 0.82];
pub const START_TS: [f64; 6] = [0.2, 0.14, 0.27, 0.34, 0.08, 0.42];
pub const END_TS: [f64; 6] = [0.8, 0.86, 0.73, 0.66, 0.92, 0.58];
/// 时序图优先靠发送端：交叉报文的交点在中部，放那里分不清是哪一条。
pub const SENDER_TS: [f64; 9] = [0.3, 0.24, 0.36, 0.18, 0.42, 0.7, 0.76, 0.5, 0.12];
/// 到别的边的距离至少是到本边的这么多倍，才算归属明确。
const OWN_MARGIN: f64 = 1.5;
const LABEL_HALF_H: f64 = 0.5;

/// 一个待放置的文字。
pub struct Soft {
    pub text: String,
    pub scale: f64,
    pub color: String,
    /// 本边的折线；用来判断归属，也是候选位置所在的路径。
    pub path: Vec<P>,
    pub ts: Vec<f64>,
    /// 优先的一侧。
    pub side: P,
    /// 不沿边移动时的固定位置（比如自环）。
    pub fixed: Option<P>,
    /// 优先压在本边上（时序图的习惯），放不下才放到边旁。
    pub inline_first: bool,
}

/// 指向某处的具名箭头，名字放不下时拉长箭头错开。
pub struct Pointer {
    pub tip: P,
    pub dir: P,
    pub name: String,
}

/// 其余已定位的标签及其引线。
#[derive(Default)]
struct Others {
    boxes: Vec<Rect>,
    leaders: Vec<(P, P)>,
}

#[derive(Clone, Copy)]
enum Tier {
    Beside,
    Inline,
    Leader(P),
}

fn seg_dist(q: P, a: P, b: P) -> f64 {
    let d = b.sub(a);
    let l2 = d.x * d.x + d.y * d.y;
    if l2 == 0.0 {
        return q.sub(a).len();
    }
    let e = q.sub(a);
    let t = ((e.x * d.x + e.y * d.y) / l2).clamp(0.0, 1.0);
    q.sub(a.add(d.mul(t))).len()
}

pub fn path_dist(q: P, path: &[P]) -> f64 {
    path.windows(2)
        .map(|s| seg_dist(q, s[0], s[1]))
        .fold(f64::INFINITY, f64::min)
}

/// 线段与矩形是否相交（Liang-Barsky）。
pub fn seg_hits(a: P, b: P, r: &Rect) -> bool {
    let d = b.sub(a);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (dp, q) in [
        (-d.x, a.x - r.x0),
        (d.x, r.x1 - a.x),
        (-d.y, a.y - r.y0),
        (d.y, r.y1 - a.y),
    ] {
        if dp == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let k = q / dp;
            if dp < 0.0 {
                t0 = t0.max(k);
            } else {
                t1 = t1.min(k);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

fn sample_points(r: &Rect) -> [P; 9] {
    let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
    [
        p(r.x0, r.y0),
        p(cx, r.y0),
        p(r.x1, r.y0),
        p(r.x0, cy),
        p(cx, cy),
        p(r.x1, cy),
        p(r.x0, r.y1),
        p(cx, r.y1),
        p(r.x1, r.y1),
    ]
}

/// 折线上按弧长比例 t 的点及该处方向。
fn along(path: &[P], t: f64) -> (P, P) {
    let total = arclen(path);
    let mut left = total * t;
    for s in path.windows(2) {
        let l = s[1].sub(s[0]).len();
        if l > 0.0 && left <= l {
            return (
                s[0].add(s[1].sub(s[0]).mul(left / l)),
                s[1].sub(s[0]).norm(),
            );
        }
        left -= l;
    }
    (path[path.len() - 1], p(1.0, 0.0))
}

fn arclen(path: &[P]) -> f64 {
    path.windows(2).map(|s| s[1].sub(s[0]).len()).sum()
}

impl Canvas<'_> {
    fn soft_box(&self, c: P, s: &Soft) -> Rect {
        Rect::around(
            c,
            self.ws(&s.text, s.scale) / 2.0,
            self.em * s.scale * LABEL_HALF_H,
        )
    }

    /// 矩形是否没有碰到别的东西。own 为 Some 时忽略落在该折线上的线段（标签压在本边上）。
    fn is_free(&self, r: &Rect, own: Option<&[P]>, others: &Others) -> bool {
        let pad = self.em * 0.12;
        let big = r.grown(pad);
        let eps = self.em * 0.15;
        let mine = |a: P, b: P| {
            own.is_some_and(|path| path_dist(a, path) < eps && path_dist(b, path) < eps)
        };
        !self
            .segs()
            .iter()
            .any(|&(a, b)| !mine(a, b) && seg_hits(a, b, &big))
            && !self.boxes().iter().any(|b| b.overlaps(&big))
            && !self.text_boxes().iter().any(|b| b.overlaps(&big))
            && !others.boxes.iter().any(|b| b.overlaps(&big))
            && !others.leaders.iter().any(|&(a, b)| seg_hits(a, b, &big))
    }

    /// 标签到本边是否明显比到别的边近。
    fn owned(&self, r: &Rect, path: &[P]) -> bool {
        let eps = self.em * 0.15;
        let pts = sample_points(r);
        let own = pts
            .iter()
            .map(|q| path_dist(*q, path))
            .fold(f64::INFINITY, f64::min);
        let other = self
            .segs()
            .iter()
            .filter(|&&(a, b)| !(path_dist(a, path) < eps && path_dist(b, path) < eps))
            .flat_map(|&(a, b)| pts.iter().map(move |q| seg_dist(*q, a, b)))
            .fold(f64::INFINITY, f64::min);
        other >= own * OWN_MARGIN
    }

    /// 找一处能压在本边上的位置。
    fn find_inline(&self, s: &Soft, others: &Others) -> Option<P> {
        if arclen(&s.path) < self.ws(&s.text, s.scale) + self.em * 1.5 {
            return None;
        }
        s.ts.iter().map(|&t| along(&s.path, t).0).find(|&c| {
            let r = self.soft_box(c, s);
            self.is_free(&r, Some(&s.path), others)
        })
    }

    fn try_place(&self, s: &Soft, others: &Others) -> (P, Tier) {
        let em = self.em;
        let w = self.ws(&s.text, s.scale);
        let centre = |q: P, n: P| {
            let ext = n.x.abs() * w / 2.0 + n.y.abs() * em * s.scale * LABEL_HALF_H;
            q.add(n.mul(em * 0.25 + ext))
        };
        if let Some(c) = s.fixed {
            let r = self.soft_box(c, s);
            if self.is_free(&r, None, others) {
                return (c, Tier::Beside);
            }
        } else {
            if s.inline_first {
                if let Some(c) = self.find_inline(s, others) {
                    return (c, Tier::Inline);
                }
            }
            for &t in &s.ts {
                let (q, dir) = along(&s.path, t);
                let n = dir.perp();
                let first = if n.x * s.side.x + n.y * s.side.y >= 0.0 {
                    n
                } else {
                    n.mul(-1.0)
                };
                for n in [first, first.mul(-1.0)] {
                    let c = centre(q, n);
                    let r = self.soft_box(c, s);
                    if self.is_free(&r, None, others) && self.owned(&r, &s.path) {
                        return (c, Tier::Beside);
                    }
                }
            }
            if let Some(c) = self.find_inline(s, others) {
                return (c, Tier::Inline);
            }
        }
        let (anchor, _) = if s.path.len() >= 2 {
            along(&s.path, s.ts.first().copied().unwrap_or(0.5))
        } else {
            (s.fixed.unwrap_or(p(0.0, 0.0)), p(1.0, 0.0))
        };
        // 由近及远找空白处；图是有限的，走到图外总能放下
        let mut radius = em * 1.2;
        loop {
            for k in 0..16 {
                let a = std::f64::consts::TAU * k as f64 / 16.0;
                let c = anchor.add(p(a.cos(), a.sin()).mul(radius + w / 2.0));
                let r = self.soft_box(c, s);
                let from = r.nearest_edge(anchor);
                let clear = |b: &Rect| !seg_hits(from, anchor, &b.grown(self.em * 0.05));
                if self.is_free(&r, None, others)
                    && others.boxes.iter().all(clear)
                    && self.text_boxes().iter().all(clear)
                {
                    return (c, Tier::Leader(anchor));
                }
            }
            radius += em * 0.6;
        }
    }

    fn place_pointers(&mut self) {
        let pointers = self.take_pointers();
        let em = self.em;
        for q in pointers {
            let d = q.dir.norm();
            let w = self.w(&q.name);
            let at = |len: f64| {
                let start = q.tip.add(d.mul(len));
                let c = if d.y.abs() >= d.x.abs() {
                    start.add(p(0.0, em * 0.75 * d.y.signum()))
                } else {
                    start.add(p((w / 2.0 + em * 0.3) * d.x.signum(), 0.0))
                };
                (start, c)
            };
            // 箭头逐级拉长，直到名字不碰别的文字和图形
            let len = (0..6)
                .map(|k| em * (1.6 + 1.25 * k as f64))
                .find(|&len| {
                    let (start, c) = at(len);
                    let r = Rect::around(c, w / 2.0, em * LABEL_HALF_H);
                    let shaft = |t: &Rect| seg_hits(start, q.tip.add(d.mul(em * 0.2)), t);
                    self.is_free(&r, None, &Others::default())
                        && !self.text_boxes().iter().any(shaft)
                })
                .unwrap_or(em * 1.6);
            let (start, c) = at(len);
            let pen = self.pen();
            self.arrow(start, q.tip, pen);
            self.label(c, &q.name, crate::svg::INK);
        }
    }

    /// 放置所有延后的文字。必须在图形全部画完后调用。
    pub fn resolve_labels(&mut self) {
        self.place_pointers();
        let softs = self.take_softs();
        let mut placed: Vec<Option<(P, Tier)>> = vec![None; softs.len()];
        // 第二遍让先放的标签也能看到后放的标签
        for _ in 0..2 {
            for (i, s) in softs.iter().enumerate() {
                let mut others = Others::default();
                for (j, (o, r)) in softs.iter().zip(&placed).enumerate() {
                    if let (true, Some((c, tier))) = (j != i, r) {
                        let b = self.soft_box(*c, o);
                        if let Tier::Leader(to) = tier {
                            others.leaders.push((b.nearest_edge(*to), *to));
                        }
                        others.boxes.push(b);
                    }
                }
                placed[i] = Some(self.try_place(s, &others));
            }
        }
        for (s, r) in softs.iter().zip(placed) {
            let (c, tier) = r.expect("every label is placed");
            let b = self.soft_box(c, s);
            match tier {
                Tier::Beside => {}
                Tier::Inline => self.mask(&b.grown(self.em * 0.08)),
                Tier::Leader(to) => {
                    self.count_leader();
                    let from = b.nearest_edge(to);
                    let pen = Pen {
                        color: MUTED,
                        width: self.sw * 0.7,
                        dash: false,
                    };
                    self.line(from, to, pen);
                }
            }
            self.text_soft(c, s);
        }
    }

    fn text_soft(&mut self, c: P, s: &Soft) {
        self.mark_soft(self.soft_box(c, s));
        self.text(c, &s.text, Anchor::Middle, s.scale, &s.color);
    }
}
