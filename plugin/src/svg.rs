//! SVG 画布、字宽表和公共绘图原语。坐标单位为 pt。

use crate::parse::{Attrs, Color};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;

pub const INK: &str = "#262626";
pub const MUTED: &str = "#8a8a8a";
pub const SUB_SCALE: f64 = 0.7;

pub struct Config {
    pub size: f64,
    pub stroke: f64,
    pub spacing: f64,
    pub font: String,
    pub widths: HashMap<String, f64>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            size: 10.0,
            stroke: 0.8,
            spacing: 1.0,
            font: "sans-serif".into(),
            widths: HashMap::new(),
        }
    }
}

impl Config {
    /// 解析 Typst 传来的配置：`键=值` 行，以及 `宽度\t文字` 行。
    pub fn parse(s: &str) -> Result<Config, String> {
        let mut c = Config::default();
        let mut stroke = None;
        for line in s.lines() {
            if let Some((w, t)) = line.split_once('\t') {
                let w: f64 = w
                    .parse()
                    .map_err(|_| format!("dsviz: malformed width entry `{}`", line))?;
                c.widths.insert(t.to_string(), w);
            } else if let Some((k, v)) = line.split_once('=') {
                let num = || {
                    v.parse::<f64>()
                        .map_err(|_| format!("dsviz: parameter {} is not a number", k))
                };
                match k {
                    "size" => c.size = num()?,
                    "stroke" => stroke = Some(num()?),
                    "spacing" => c.spacing = num()?,
                    "font" => c.font = v.to_string(),
                    _ => {}
                }
            }
        }
        c.stroke = stroke.unwrap_or(c.size * 0.08);
        Ok(c)
    }
}

/// 把 `a_1` 拆成正文和下标；`\_` 表示普通下划线。
pub fn split_sub(s: &str) -> (String, String) {
    let mut base = String::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '\\' && cs.get(i + 1) == Some(&'_') {
            base.push('_');
            i += 2;
            continue;
        }
        if cs[i] == '_' && i > 0 && i + 1 < cs.len() {
            let mut sub: String = cs[i + 1..].iter().collect();
            if sub.starts_with('{') && sub.ends_with('}') && sub.len() >= 2 {
                sub = sub[1..sub.len() - 1].to_string();
            }
            return (base, sub.replace("\\_", "_"));
        }
        base.push(cs[i]);
        i += 1;
    }
    (base, String::new())
}

/// 字宽表。第一遍运行时记录所有要量的字符串，第二遍使用 Typst 量出的宽度。
pub struct Meas {
    size: f64,
    widths: HashMap<String, f64>,
    record: Option<RefCell<Vec<String>>>,
}

impl Meas {
    pub fn recording(size: f64) -> Meas {
        Meas {
            size,
            widths: HashMap::new(),
            record: Some(RefCell::new(Vec::new())),
        }
    }
    pub fn from_cfg(c: &Config) -> Meas {
        Meas {
            size: c.size,
            widths: c.widths.clone(),
            record: None,
        }
    }
    pub fn recorded(&self) -> Vec<String> {
        self.record
            .as_ref()
            .map(|r| r.borrow().clone())
            .unwrap_or_default()
    }

    fn raw(&self, s: &str) -> f64 {
        if s.is_empty() {
            return 0.0;
        }
        if let Some(r) = &self.record {
            let mut r = r.borrow_mut();
            if !r.iter().any(|x| x == s) {
                r.push(s.to_string());
            }
        }
        if let Some(w) = self.widths.get(s) {
            return *w;
        }
        // 没有实测值时的估算，只在第一遍使用。
        s.chars()
            .map(|c| if (c as u32) > 0x2E80 { 1.0 } else { 0.6 })
            .sum::<f64>()
            * self.size
    }

    /// 标签按正常字号排版时的宽度（含下标）。
    pub fn width(&self, s: &str) -> f64 {
        let (b, sub) = split_sub(s);
        self.raw(&b) + SUB_SCALE * self.raw(&sub)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P {
    pub x: f64,
    pub y: f64,
}

pub fn p(x: f64, y: f64) -> P {
    P { x, y }
}

impl P {
    pub fn add(self, o: P) -> P {
        p(self.x + o.x, self.y + o.y)
    }
    pub fn sub(self, o: P) -> P {
        p(self.x - o.x, self.y - o.y)
    }
    pub fn mul(self, k: f64) -> P {
        p(self.x * k, self.y * k)
    }
    pub fn len(self) -> f64 {
        self.x.hypot(self.y)
    }
    pub fn norm(self) -> P {
        let l = self.len();
        if l < 1e-9 {
            p(0.0, 1.0)
        } else {
            self.mul(1.0 / l)
        }
    }
    /// 左法向（y 轴向下时为顺时针 90°）。
    pub fn perp(self) -> P {
        p(self.y, -self.x)
    }
    pub fn mid(self, o: P) -> P {
        p((self.x + o.x) / 2.0, (self.y + o.y) / 2.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle(f64),
    Rect { hw: f64, hh: f64, rx: f64 },
    Point,
}

impl Shape {
    pub fn half(&self) -> (f64, f64) {
        match *self {
            Shape::Circle(r) => (r, r),
            Shape::Rect { hw, hh, .. } => (hw, hh),
            Shape::Point => (0.0, 0.0),
        }
    }

    /// 从中心 c 沿方向射向 toward 时与边界的交点。
    pub fn boundary(&self, c: P, toward: P) -> P {
        let d = toward.sub(c).norm();
        match *self {
            Shape::Point => c,
            Shape::Circle(r) => c.add(d.mul(r)),
            Shape::Rect { hw, hh, rx } => {
                let tx = if d.x.abs() > 1e-9 {
                    hw / d.x.abs()
                } else {
                    f64::INFINITY
                };
                let ty = if d.y.abs() > 1e-9 {
                    hh / d.y.abs()
                } else {
                    f64::INFINITY
                };
                let t = tx.min(ty);
                let hit = d.mul(t);
                let (cx, cy) = (hw - rx, hh - rx);
                if rx > 0.0 && hit.x.abs() > cx && hit.y.abs() > cy {
                    // 落在圆角上：与圆角圆求交。
                    let o = p(cx * hit.x.signum(), cy * hit.y.signum());
                    let b = d.x * o.x + d.y * o.y;
                    let cc = o.x * o.x + o.y * o.y - rx * rx;
                    let disc = (b * b - cc).max(0.0);
                    let t2 = b + disc.sqrt();
                    return c.add(d.mul(t2));
                }
                c.add(hit)
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Pen {
    pub color: &'static str,
    pub width: f64,
    pub dash: bool,
}

/// 元素着色：(填充, 描边, 文字)。
pub fn paint(c: Option<Color>) -> (Option<&'static str>, &'static str, &'static str) {
    match c {
        None => (None, INK, INK),
        Some(Color::Hl) => (Some("#fff0a0"), INK, INK),
        Some(Color::Red) => (Some("#fbdcd8"), "#c0392b", INK),
        Some(Color::Green) => (Some("#dcf0d8"), "#2e7d32", INK),
        Some(Color::Blue) => (Some("#dce8f6"), "#2f6db5", INK),
        Some(Color::Gray) => (Some("#ececec"), MUTED, "#666666"),
    }
}

/// 线条颜色。
pub fn line_color(c: Option<Color>) -> &'static str {
    match c {
        None => INK,
        Some(Color::Hl) => "#d19a00",
        Some(Color::Red) => "#c0392b",
        Some(Color::Green) => "#2e7d32",
        Some(Color::Blue) => "#2f6db5",
        Some(Color::Gray) => "#a0a0a0",
    }
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            _ => o.push(c),
        }
    }
    o
}

fn f(x: f64) -> String {
    let r = (x * 100.0).round() / 100.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        format!("{}", r)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

pub struct Canvas<'a> {
    pub cfg: &'a Config,
    pub m: &'a Meas,
    /// 字号 em。
    pub em: f64,
    /// 线宽。
    pub sw: f64,
    /// 间距系数。
    pub sp: f64,
    buf: String,
    bb: [f64; 4],
    texts: Vec<TextItem>,
}

/// 文字不写进 SVG，而是交给 Typst 排版，保证字体与实测宽度一致。
struct TextItem {
    x: f64,
    y: f64,
    anchor: Anchor,
    scale: f64,
    color: String,
    text: String,
}

impl<'a> Canvas<'a> {
    pub fn new(cfg: &'a Config, m: &'a Meas) -> Canvas<'a> {
        Canvas {
            cfg,
            m,
            em: cfg.size,
            sw: cfg.stroke,
            sp: cfg.spacing,
            buf: String::new(),
            bb: [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            texts: Vec::new(),
        }
    }

    pub fn w(&self, s: &str) -> f64 {
        self.m.width(s)
    }

    /// 按比例缩放的字宽（比如小号字）。
    pub fn ws(&self, s: &str, scale: f64) -> f64 {
        self.m.width(s) * scale
    }

    pub fn grow(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.bb[0] = self.bb[0].min(x0.min(x1));
        self.bb[1] = self.bb[1].min(y0.min(y1));
        self.bb[2] = self.bb[2].max(x0.max(x1));
        self.bb[3] = self.bb[3].max(y0.max(y1));
    }

    fn grow_p(&mut self, q: P, pad: f64) {
        self.grow(q.x - pad, q.y - pad, q.x + pad, q.y + pad);
    }

    pub fn pen(&self) -> Pen {
        Pen {
            color: INK,
            width: self.sw,
            dash: false,
        }
    }

    fn stroke_attr(&self, pen: Pen) -> String {
        let mut s = format!(r#" stroke="{}" stroke-width="{}""#, pen.color, f(pen.width));
        if pen.dash {
            let _ = write!(
                s,
                r#" stroke-dasharray="{} {}""#,
                f(pen.width * 4.0),
                f(pen.width * 3.0)
            );
        }
        s
    }

    pub fn raw(&mut self, s: &str) {
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    pub fn line(&mut self, a: P, b: P, pen: Pen) {
        self.grow_p(a, pen.width);
        self.grow_p(b, pen.width);
        let s = format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{} stroke-linecap="round"/>"#,
            f(a.x),
            f(a.y),
            f(b.x),
            f(b.y),
            self.stroke_attr(pen)
        );
        self.raw(&s);
    }

    pub fn polyline(&mut self, pts: &[P], pen: Pen) {
        let mut d = String::new();
        for (i, q) in pts.iter().enumerate() {
            self.grow_p(*q, pen.width);
            let _ = write!(
                d,
                "{}{} {} ",
                if i == 0 { "M" } else { "L" },
                f(q.x),
                f(q.y)
            );
        }
        let s = format!(
            r#"<path d="{}" fill="none"{} stroke-linejoin="round"/>"#,
            d.trim(),
            self.stroke_attr(pen)
        );
        self.raw(&s);
    }

    pub fn rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        rx: f64,
        fill: Option<&str>,
        stroke: Option<Pen>,
    ) {
        let pad = stroke.map(|p| p.width).unwrap_or(0.0);
        self.grow(x - pad, y - pad, x + w + pad, y + h + pad);
        let st = stroke.map(|p| self.stroke_attr(p)).unwrap_or_default();
        let s = format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}"{}/>"#,
            f(x),
            f(y),
            f(w),
            f(h),
            f(rx),
            fill.unwrap_or("none"),
            st
        );
        self.raw(&s);
    }

    pub fn circle(&mut self, c: P, r: f64, fill: Option<&str>, stroke: Option<Pen>) {
        let pad = stroke.map(|p| p.width).unwrap_or(0.0);
        self.grow_p(c, r + pad);
        let st = stroke.map(|p| self.stroke_attr(p)).unwrap_or_default();
        let s = format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}"{}/>"#,
            f(c.x),
            f(c.y),
            f(r),
            fill.unwrap_or("none"),
            st
        );
        self.raw(&s);
    }

    pub fn path(&mut self, d: &str, fill: Option<&str>, stroke: Option<Pen>, pts: &[P]) {
        let pad = stroke.map(|p| p.width).unwrap_or(0.0);
        for q in pts {
            self.grow_p(*q, pad);
        }
        let st = stroke.map(|p| self.stroke_attr(p)).unwrap_or_default();
        let s = format!(
            r#"<path d="{}" fill="{}"{} stroke-linejoin="round"/>"#,
            d,
            fill.unwrap_or("none"),
            st
        );
        self.raw(&s);
    }

    /// 画一行文字，c 为中心点（Middle）或左/右端点；scale 为相对字号。
    pub fn text(&mut self, c: P, s: &str, anchor: Anchor, scale: f64, color: &str) {
        if s.is_empty() {
            return;
        }
        let size = self.em * scale;
        let w = self.ws(s, scale);
        let x0 = match anchor {
            Anchor::Start => c.x,
            Anchor::Middle => c.x - w / 2.0,
            Anchor::End => c.x - w,
        };
        self.grow(x0, c.y - size * 0.6, x0 + w, c.y + size * 0.6);
        self.texts.push(TextItem {
            x: c.x,
            y: c.y,
            anchor,
            scale,
            color: color.to_string(),
            text: s.to_string(),
        });
    }

    pub fn label(&mut self, c: P, s: &str, color: &str) {
        self.text(c, s, Anchor::Middle, 1.0, color);
    }

    /// 实心箭头，tip 为尖端，dir 为前进方向。
    pub fn head(&mut self, tip: P, dir: P, color: &str) {
        let d = dir.norm();
        let n = d.perp();
        let len = self.head_len();
        let hw = len * 0.4;
        let base = tip.sub(d.mul(len));
        let a = base.add(n.mul(hw));
        let b = base.sub(n.mul(hw));
        let back = tip.sub(d.mul(len * 0.75));
        let dd = format!(
            "M{} {} L{} {} L{} {} L{} {} Z",
            f(tip.x),
            f(tip.y),
            f(a.x),
            f(a.y),
            f(back.x),
            f(back.y),
            f(b.x),
            f(b.y)
        );
        self.path(&dd, Some(color), None, &[tip, a, b]);
    }

    pub fn head_len(&self) -> f64 {
        (self.em * 0.5).max(self.sw * 4.0)
    }

    /// 带箭头的直线，箭头在 b 端。
    pub fn arrow(&mut self, a: P, b: P, pen: Pen) {
        let d = b.sub(a);
        let short = b.sub(d.norm().mul(self.head_len() * 0.6));
        self.line(a, short, pen);
        self.head(b, d, pen.color);
    }

    pub fn arrow_poly(&mut self, pts: &[P], pen: Pen) {
        let n = pts.len();
        let mut v = pts.to_vec();
        let d = pts[n - 1].sub(pts[n - 2]);
        v[n - 1] = pts[n - 1].sub(d.norm().mul(self.head_len() * 0.6));
        self.polyline(&v, pen);
        self.head(pts[n - 1], d, pen.color);
    }

    /// 三次贝塞尔曲线，可选末端箭头。
    pub fn curve(&mut self, a: P, c1: P, c2: P, b: P, pen: Pen, head: bool) {
        let mut end = b;
        let dir = b.sub(c2);
        let dir = if dir.len() < 1e-6 { b.sub(c1) } else { dir };
        if head {
            end = b.sub(dir.norm().mul(self.head_len() * 0.6));
        }
        let d = format!(
            "M{} {} C{} {} {} {} {} {}",
            f(a.x),
            f(a.y),
            f(c1.x),
            f(c1.y),
            f(c2.x),
            f(c2.y),
            f(end.x),
            f(end.y)
        );
        // 包围盒用曲线上的采样点。
        let mut pts = Vec::new();
        for i in 0..=8 {
            pts.push(bez(a, c1, c2, b, i as f64 / 8.0));
        }
        self.path(&d, None, Some(pen), &pts);
        if head {
            self.head(b, dir, pen.color);
        }
    }

    /// 穿过各点的平滑曲线（Catmull-Rom），可选两端箭头。
    pub fn spline(&mut self, pts: &[P], pen: Pen, head_end: bool, head_start: bool) {
        let n = pts.len();
        let mut q = pts.to_vec();
        let hl = self.head_len() * 0.6;
        if head_end {
            q[n - 1] = pts[n - 1].sub(pts[n - 1].sub(pts[n - 2]).norm().mul(hl));
        }
        if head_start {
            q[0] = pts[0].sub(pts[0].sub(pts[1]).norm().mul(hl));
        }
        let mut d = format!("M{} {}", f(q[0].x), f(q[0].y));
        for i in 0..n - 1 {
            let p0 = q[i.saturating_sub(1)];
            let (p1, p2) = (q[i], q[i + 1]);
            let p3 = q[(i + 2).min(n - 1)];
            let c1 = p1.add(p2.sub(p0).mul(1.0 / 6.0));
            let c2 = p2.sub(p3.sub(p1).mul(1.0 / 6.0));
            let _ = write!(
                d,
                " C{} {} {} {} {} {}",
                f(c1.x),
                f(c1.y),
                f(c2.x),
                f(c2.y),
                f(p2.x),
                f(p2.y)
            );
        }
        self.path(&d, None, Some(pen), pts);
        if head_end {
            self.head(pts[n - 1], pts[n - 1].sub(pts[n - 2]), pen.color);
        }
        if head_start {
            self.head(pts[0], pts[0].sub(pts[1]), pen.color);
        }
    }

    /// 从 a 到 b 的弧形箭头，bend 为相对弦长的偏移（正值偏向前进方向左侧）。
    pub fn bend_arrow(&mut self, a: P, b: P, bend: f64, pen: Pen, head: bool) {
        let d = b.sub(a);
        let off = d.perp().mul(bend);
        let c1 = a.add(d.mul(0.25)).add(off);
        let c2 = a.add(d.mul(0.75)).add(off);
        self.curve(a, c1, c2, b, pen, head);
    }

    /// 结点：圆或圆角矩形，带着色与文字。
    pub fn node(&mut self, c: P, shape: Shape, text: &str, attrs: &Attrs) {
        let (fill, stroke, tc) = paint(attrs.color);
        let pen = Pen {
            color: stroke,
            width: self.sw,
            dash: attrs.dashed,
        };
        self.shape(c, shape, fill, Some(pen));
        self.label(c, text, tc);
    }

    pub fn shape(&mut self, c: P, shape: Shape, fill: Option<&str>, pen: Option<Pen>) {
        match shape {
            Shape::Circle(r) => self.circle(c, r, fill, pen),
            Shape::Rect { hw, hh, rx } => {
                self.rect(c.x - hw, c.y - hh, hw * 2.0, hh * 2.0, rx, fill, pen)
            }
            Shape::Point => {}
        }
    }

    /// 圆形结点的默认形状：短标签用圆，长标签用胶囊形。
    pub fn node_shape(&self, text: &str) -> Shape {
        let w = self.w(text);
        let r = self.em * 0.9;
        if w <= self.em * 1.3 {
            Shape::Circle(r.max(w / 2.0 + self.em * 0.3))
        } else {
            Shape::Rect {
                hw: w / 2.0 + self.em * 0.55,
                hh: r,
                rx: r,
            }
        }
    }

    /// 元素的结点形状：`[box]` 为方框，否则按 node_shape。
    pub fn elem_shape(&self, e: &crate::parse::Elem) -> Shape {
        if e.attrs.boxed {
            let w = self.w(e.label());
            Shape::Rect {
                hw: (w / 2.0 + self.em * 0.45).max(self.em * 0.8),
                hh: self.em * 0.8,
                rx: 0.0,
            }
        } else {
            self.node_shape(e.label())
        }
    }

    /// 边标签：画在边 a→b 中点外侧，right 为真放右侧。
    pub fn edge_label(&mut self, a: P, b: P, s: &str, right: bool) {
        let m = a.mid(b);
        let d = b.sub(a).norm();
        let mut n = d.perp();
        if (n.x < 0.0) == right {
            n = n.mul(-1.0);
        }
        let w = self.ws(s, 0.8);
        let ext = n.x.abs() * w / 2.0 + n.y.abs() * self.em * 0.4;
        self.text(
            m.add(n.mul(self.em * 0.25 + ext)),
            s,
            Anchor::Middle,
            0.8,
            INK,
        );
    }

    /// 指针标注：从 tip 出发沿 dir 反方向画一段带名字的箭头。
    pub fn pointer(&mut self, tip: P, from_dir: P, names: &[String]) {
        if names.is_empty() {
            return;
        }
        let name = names.join(", ");
        let d = from_dir.norm();
        let len = self.em * 1.6;
        let start = tip.add(d.mul(len));
        let pen = Pen {
            color: INK,
            width: self.sw,
            dash: false,
        };
        self.arrow(start, tip, pen);
        let gap = self.em * 0.75;
        let w = self.w(&name);
        // 名字放在起点外侧。
        let lc = if d.y.abs() >= d.x.abs() {
            start.add(p(0.0, gap * d.y.signum()))
        } else {
            start.add(p((w / 2.0 + self.em * 0.3) * d.x.signum(), 0.0))
        };
        self.label(lc, &name, INK);
    }

    /// 输出格式：首行 `宽 高`，随后每行一条文字
    /// `x\ty\t对齐\t缩放\t颜色\t正文\t下标`，再接 `\u{1}` 和 SVG。
    pub fn finish(self) -> String {
        let (mut x0, mut y0, mut x1, mut y1) = (self.bb[0], self.bb[1], self.bb[2], self.bb[3]);
        if !x0.is_finite() {
            (x0, y0, x1, y1) = (0.0, 0.0, 1.0, 1.0);
        }
        let m = self.sw * 1.5 + 0.5;
        x0 -= m;
        y0 -= m;
        x1 += m;
        y1 += m;
        let (w, h) = (x1 - x0, y1 - y0);
        let mut out = format!("{} {}\n", f(w), f(h));
        for t in &self.texts {
            let (base, sub) = split_sub(&t.text);
            let a = match t.anchor {
                Anchor::Start => "start",
                Anchor::Middle => "middle",
                Anchor::End => "end",
            };
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                f(t.x - x0),
                f(t.y - y0),
                a,
                t.scale,
                t.color,
                base,
                sub
            );
        }
        out.push('\u{1}');
        let _ = write!(
            out,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="{x} {y} {w} {h}">
{body}</svg>"#,
            w = f(w),
            h = f(h),
            x = f(x0),
            y = f(y0),
            body = self.buf
        );
        out
    }
}

pub fn bez(a: P, c1: P, c2: P, b: P, t: f64) -> P {
    let u = 1.0 - t;
    a.mul(u * u * u)
        .add(c1.mul(3.0 * u * u * t))
        .add(c2.mul(3.0 * u * t * t))
        .add(b.mul(t * t * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscripts() {
        assert_eq!(split_sub("a_1"), ("a".into(), "1".into()));
        assert_eq!(split_sub("x_{ij}"), ("x".into(), "ij".into()));
        assert_eq!(split_sub("_"), ("_".into(), "".into()));
        assert_eq!(split_sub("a\\_b"), ("a_b".into(), "".into()));
        assert_eq!(split_sub("a_"), ("a_".into(), "".into()));
    }

    #[test]
    fn rect_boundary() {
        let s = Shape::Rect {
            hw: 10.0,
            hh: 5.0,
            rx: 0.0,
        };
        let b = s.boundary(p(0.0, 0.0), p(100.0, 0.0));
        assert!((b.x - 10.0).abs() < 1e-9);
        let b = s.boundary(p(0.0, 0.0), p(0.0, -100.0));
        assert!((b.y + 5.0).abs() < 1e-9);
    }

    #[test]
    fn meas_records() {
        let m = Meas::recording(10.0);
        m.width("a_1");
        m.width("a");
        assert_eq!(m.recorded(), vec!["a".to_string(), "1".to_string()]);
    }
}
