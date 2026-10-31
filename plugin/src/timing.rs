//! fields（位段）/ pipeline（流水线）/ gantt（甘特图）/ wave（波形）/ ladder（时序图）

use crate::parse::{err, fail, Attrs, Doc, Elem, Line, Res, Tok};
use crate::place::{Soft, SENDER_TS};
use crate::svg::{line_color, line_width, p, paint, Anchor, Canvas, Pen, INK, MUTED, P};

/// 小号标注的颜色：比 MUTED 深，黑白打印也看得清。
const NOTE: &str = "#555555";
const GRID: &str = "#b8b8b8";
const STALL: &str = "#d4d4d4";

// ---------------- 公共 ----------------

/// 取行标题：`名字:`，或 `"带 空格":`。
pub(crate) fn row_head(l: &Line) -> (Option<String>, Vec<Tok>) {
    if let (Some(Tok::E(a)), Some(Tok::E(b))) = (l.toks.first(), l.toks.get(1)) {
        if a.quoted && !b.quoted && (b.text == ":" || b.text == "：") {
            return (Some(a.text.clone()), l.toks[2..].to_vec());
        }
    }
    l.split_row_label()
}

/// 闭合多边形。
pub(crate) fn polygon(cv: &mut Canvas, pts: &[P], fill: Option<&str>, pen: Option<Pen>) {
    let mut d = String::new();
    for (i, q) in pts.iter().enumerate() {
        d.push_str(&format!(
            "{}{:.2} {:.2} ",
            if i == 0 { "M" } else { "L" },
            q.x,
            q.y
        ));
    }
    d.push('Z');
    cv.path(&d, fill, pen, pts);
}

/// 去掉重合点和共线的中间点。
pub(crate) fn clean(pts: &[P]) -> Vec<P> {
    let mut v: Vec<P> = Vec::new();
    for q in pts {
        if v.last().map_or(true, |l| l.sub(*q).len() > 0.01) {
            v.push(*q);
        }
    }
    let mut i = 1;
    while i + 1 < v.len() {
        let (a, b) = (v[i].sub(v[i - 1]), v[i + 1].sub(v[i]));
        if (a.x * b.y - a.y * b.x).abs() < 1e-6 && a.x * b.x + a.y * b.y > 0.0 {
            v.remove(i);
        } else {
            i += 1;
        }
    }
    v
}

/// 矩形内 45° 斜线；both 为真时画成交叉阴影。
fn hatch(cv: &mut Canvas, x: f64, y: f64, w: f64, h: f64, step: f64, both: bool, pen: Pen) {
    let mut c = step;
    while c < w + h {
        let (ax, ay) = (c.min(w), (c - w).max(0.0));
        let (bx, by) = ((c - h).max(0.0), c.min(h));
        cv.line(p(x + ax, y + ay), p(x + bx, y + by), pen);
        if both {
            cv.line(p(x + w - ax, y + ay), p(x + w - bx, y + by), pen);
        }
        c += step;
    }
}

fn row_title(cv: &mut Canvas, s: &Option<String>, y: f64) {
    if let Some(s) = s {
        let x = -cv.em * 0.6;
        cv.text(p(x, y), s, Anchor::End, 1.0, INK);
    }
}

fn cell_pen(cv: &Canvas, a: &Attrs) -> Pen {
    Pen {
        color: paint(a.color).1,
        width: cv.sw,
        dash: a.dashed,
    }
}

fn line_pen(cv: &Canvas, a: &Attrs, k: f64) -> Pen {
    Pen {
        color: line_color(a.color),
        width: line_width(a.color, cv.sw * k),
        dash: a.dashed,
    }
}

fn parse_num(s: &str, no: usize, what: &str) -> Res<f64> {
    match s.trim().parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => err(no, format!("{} `{}` is not a number", what, s)),
    }
}

fn fmt_num(t: f64) -> String {
    if t == t.trunc() {
        format!("{}", t as i64)
    } else {
        format!("{}", t)
    }
}

// ---------------- fields ----------------

struct Field {
    name: String,
    bits: u32,
    attrs: Attrs,
}

fn parse_fields(l: &Line, toks: &[Tok]) -> Res<Vec<Field>> {
    // 先按 `|` 分组，`a:1|b:2` 粘连写法也算。
    let mut groups: Vec<Vec<Elem>> = vec![Vec::new()];
    for t in toks {
        match t {
            Tok::E(e) if e.quoted => groups.last_mut().unwrap().push(e.clone()),
            Tok::E(e) => {
                let parts: Vec<&str> = e.text.split('|').collect();
                let last = parts.len() - 1;
                for (i, s) in parts.iter().enumerate() {
                    if i > 0 {
                        groups.push(Vec::new());
                    }
                    if !s.is_empty() {
                        let mut x = e.clone();
                        x.text = s.to_string();
                        if i != last {
                            x.attrs = Attrs::default();
                        }
                        groups.last_mut().unwrap().push(x);
                    }
                }
            }
            Tok::Attrs(a) => match groups.last_mut().unwrap().last_mut() {
                Some(e) => e.attrs.merge(a.clone()),
                None => return err(l.no, "attributes `[...]` must follow a field"),
            },
            _ => return err(l.no, "a field is written `name:bits`, separated by `|`"),
        }
    }
    let mut out = Vec::new();
    for g in groups {
        let full = g
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let mut attrs = Attrs::default();
        for e in g {
            attrs.merge(e.attrs);
        }
        let (name, bits) = match full.rfind([':', '：']) {
            Some(i) => {
                let c = full[i..].chars().next().unwrap();
                (full[..i].trim(), &full[i + c.len_utf8()..])
            }
            None => {
                return err(
                    l.no,
                    format!("field `{}` is missing its width, write `name:bits`", full),
                )
            }
        };
        let bits: u32 = match bits.trim().parse() {
            Ok(b) if b > 0 => b,
            _ => {
                return err(
                    l.no,
                    format!(
                        "field `{}` has invalid width `{}`; it must be a positive integer",
                        name, bits
                    ),
                )
            }
        };
        out.push(Field {
            name: if name == "_" {
                String::new()
            } else {
                name.to_string()
            },
            bits,
            attrs,
        });
    }
    Ok(out)
}

/// 把总宽 total 分给各格：按位数比例，但不小于各自最小宽度。
fn fit(bits: &[f64], min: &[f64], total: f64) -> Vec<f64> {
    let mut w = vec![None; bits.len()];
    loop {
        let fixed: f64 = w.iter().flatten().sum();
        let free: f64 = (0..bits.len())
            .filter(|&i| w[i].is_none())
            .map(|i| bits[i])
            .sum();
        if free <= 0.0 {
            break;
        }
        let k = (total - fixed) / free;
        let mut changed = false;
        for i in 0..bits.len() {
            if w[i].is_none() && bits[i] * k < min[i] {
                w[i] = Some(min[i]);
                changed = true;
            }
        }
        if !changed {
            for i in 0..bits.len() {
                if w[i].is_none() {
                    w[i] = Some(bits[i] * k);
                }
            }
            break;
        }
    }
    w.into_iter().map(|x| x.unwrap_or(0.0)).collect()
}

pub fn fields(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["ticks"])?;
    let ticks = doc.opt_bool("ticks", false)?;
    if doc.lines.is_empty() {
        return err(doc.head_line, "nothing to draw");
    }
    let em = cv.em;
    let small = 0.75;
    let mut rows = Vec::new();
    for l in &doc.lines {
        let (title, toks) = row_head(l);
        if toks.is_empty() {
            return err(l.no, "this row has no fields");
        }
        rows.push((title, parse_fields(l, &toks)?));
    }
    // 每格的最小宽度，以及放得下全部格子的公共比例尺。
    let longest: u32 = rows
        .iter()
        .map(|(_, fs)| fs.iter().map(|f| f.bits).sum())
        .max()
        .unwrap_or(1);
    let mut scale = (em * 0.85).min(em * 36.0 / longest as f64) * cv.sp.max(0.5);
    let mut mins: Vec<Vec<f64>> = Vec::new();
    for (_, fs) in &rows {
        let total: u32 = fs.iter().map(|f| f.bits).sum();
        let mut hi = total;
        let mut m = Vec::new();
        for f in fs {
            let top = hi - 1;
            let low = hi - f.bits;
            hi = low;
            let tick = if !ticks {
                cv.ws(&f.bits.to_string(), small) + em * 0.3
            } else if f.bits > 1 {
                cv.ws(&top.to_string(), small) + cv.ws(&low.to_string(), small) + em * 0.6
            } else {
                cv.ws(&top.to_string(), small) + em * 0.3
            };
            m.push((cv.w(&f.name) + em * 0.8).max(tick).max(em * 1.2));
        }
        scale = scale.max(m.iter().sum::<f64>() / total as f64);
        mins.push(m);
    }
    let (above, ch, gap) = (em * 1.0, em * 1.9, em * 0.6 * cv.sp.max(0.5));
    let mut y = 0.0;
    for ((title, fs), min) in rows.iter().zip(&mins) {
        let bits: Vec<f64> = fs.iter().map(|f| f.bits as f64).collect();
        let total: f64 = bits.iter().sum();
        let ws = fit(&bits, min, total * scale);
        let top = y + above;
        row_title(cv, title, top + ch / 2.0);
        let (mut x, mut hi) = (0.0, total as u32);
        for (f, w) in fs.iter().zip(&ws) {
            let (fill, _, tc) = paint(f.attrs.color);
            let pen = cell_pen(cv, &f.attrs);
            cv.rect(x, top, *w, ch, 0.0, fill, Some(pen));
            cv.label(p(x + w / 2.0, top + ch / 2.0), &f.name, tc);
            let ty = y + above * 0.45;
            let (a, b) = (hi - 1, hi - f.bits);
            hi = b;
            if !ticks {
                let s = f.bits.to_string();
                cv.text(p(x + w / 2.0, ty), &s, Anchor::Middle, small, NOTE);
            } else if f.bits == 1 {
                cv.text(
                    p(x + w / 2.0, ty),
                    &a.to_string(),
                    Anchor::Middle,
                    small,
                    NOTE,
                );
            } else {
                let pad = em * 0.15;
                cv.text(p(x + pad, ty), &a.to_string(), Anchor::Start, small, NOTE);
                cv.text(p(x + w - pad, ty), &b.to_string(), Anchor::End, small, NOTE);
            }
            x += w;
        }
        y = top + ch + gap;
    }
    Ok(())
}

// ---------------- pipeline ----------------

struct PipeRow {
    name: String,
    cells: Vec<Elem>,
}

struct Forward {
    from: (usize, usize),
    to: (usize, usize),
    attrs: Attrs,
}

fn is_stall(e: &Elem) -> bool {
    e.is("stall") || e.is("*")
}

fn pipe_ref(rows: &[PipeRow], e: &Elem, no: usize) -> Res<(usize, usize)> {
    let (name, stage) = e.text.rsplit_once('.').ok_or_else(|| {
        fail(
            no,
            format!("`{}` should be written instruction.stage", e.text),
        )
    })?;
    let r = rows.iter().position(|x| x.name == name).ok_or_else(|| {
        fail(
            no,
            format!(
                "unknown instruction `{}` (available: {})",
                name,
                rows.iter()
                    .map(|x| x.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        )
    })?;
    let c = rows[r]
        .cells
        .iter()
        .position(|x| !x.is_blank() && !is_stall(x) && x.text == stage)
        .ok_or_else(|| fail(no, format!("instruction {} has no stage `{}`", name, stage)))?;
    Ok((r, c))
}

/// 在某条缝隙里挑一条不与已有水平段相碰的车道。
fn lane(lanes: &mut Vec<Vec<(f64, f64)>>, a: f64, b: f64, m: f64) -> usize {
    let (lo, hi) = (a.min(b) - m, a.max(b) + m);
    for (i, l) in lanes.iter_mut().enumerate() {
        if l.iter().all(|&(x, y)| hi < x || lo > y) {
            l.push((lo, hi));
            return i;
        }
    }
    lanes.push(vec![(lo, hi)]);
    lanes.len() - 1
}

pub fn pipeline(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["cycles"])?;
    let mut rows: Vec<PipeRow> = Vec::new();
    let mut raw: Vec<&Line> = Vec::new();
    for l in &doc.lines {
        if l.toks.iter().any(|t| matches!(t, Tok::Arrow)) {
            raw.push(l);
            continue;
        }
        let (title, toks) = row_head(l);
        let name = match title {
            Some(n) => n,
            None => {
                return err(
                    l.no,
                    "a row is written `name: cells...`; a forward is written `I1.EX -> I2.EX`",
                )
            }
        };
        if rows.iter().any(|r| r.name == name) {
            return err(l.no, format!("duplicate instruction `{}`", name));
        }
        let cells = Line {
            no: l.no,
            indent: 0,
            toks,
        }
        .elems()?;
        rows.push(PipeRow { name, cells });
    }
    if rows.is_empty() {
        return err(doc.head_line, "nothing to draw");
    }
    let longest = rows.iter().map(|r| r.cells.len()).max().unwrap();
    let cycles = match doc.opt_num("cycles")? {
        Some(n) if n <= 0 => return err(doc.head_line, "cycles must be positive"),
        Some(n) => n as usize,
        None => longest,
    }
    .max(1);
    if longest > cycles {
        return err(
            doc.head_line,
            format!("cycles={} but a row has {} cells", cycles, longest),
        );
    }
    let mut fwds = Vec::new();
    for l in raw {
        let bad = || fail(l.no, "a forward is written `I1.EX -> I2.EX [attrs]`");
        let (a, b) = match (l.toks.first(), l.toks.get(1), l.toks.get(2)) {
            (Some(Tok::E(a)), Some(Tok::Arrow), Some(Tok::E(b))) => (a, b),
            _ => return Err(bad()),
        };
        let mut attrs = a.attrs.clone();
        attrs.merge(b.attrs.clone());
        for t in &l.toks[3..] {
            match t {
                Tok::Attrs(x) => attrs.merge(x.clone()),
                _ => return Err(bad()),
            }
        }
        fwds.push(Forward {
            from: pipe_ref(&rows, a, l.no)?,
            to: pipe_ref(&rows, b, l.no)?,
            attrs,
        });
    }

    let em = cv.em;
    let maxw = rows
        .iter()
        .flat_map(|r| &r.cells)
        .filter(|e| !is_stall(e))
        .map(|e| cv.w(e.label()))
        .fold(0.0, f64::max);
    let cw = (maxw + em * 0.7).max(em * 2.0) * cv.sp.max(0.5);
    let (rh, head) = (em * 1.7, em * 1.5);
    let n = rows.len();
    let cx = |c: usize| (c as f64 + 0.5) * cw;

    // 箭头走行间缝隙，跨多行时沿列边界下行，不压格内文字。
    struct Route {
        sx: f64,
        tx: f64,
        xb: Option<f64>,
        l1: (usize, usize),
        l2: (usize, usize),
        down: i32,
    }
    let mut lanes: Vec<Vec<Vec<(f64, f64)>>> = vec![Vec::new(); n];
    let mut routes = Vec::new();
    let margin = em * 0.15;
    for f in &fwds {
        let ((r1, c1), (r2, c2)) = (f.from, f.to);
        let (sx, tx) = (cx(c1), cx(c2));
        let down = (r2 as i32 - r1 as i32).signum();
        let (g1, g2) = match down {
            1 => (r1, r2 - 1),
            -1 => (r1 - 1, r2),
            _ => (r1, r1),
        };
        if g1 == g2 {
            let l = lane(&mut lanes[g1], sx, tx, margin);
            routes.push(Route {
                sx,
                tx,
                xb: None,
                l1: (g1, l),
                l2: (g1, l),
                down,
            });
        } else {
            let xb = if c2 >= c1 {
                c2 as f64 * cw
            } else {
                (c2 + 1) as f64 * cw
            };
            let a = lane(&mut lanes[g1], sx, xb, margin);
            let b = lane(&mut lanes[g2], xb, tx, margin);
            routes.push(Route {
                sx,
                tx,
                xb: Some(xb),
                l1: (g1, a),
                l2: (g2, b),
                down,
            });
        }
    }
    let gap_of = |g: usize| {
        let k = lanes[g].len();
        if k == 0 {
            em * 0.3
        } else {
            em * (0.35 + 0.3 * (k as f64 - 1.0) + 0.65)
        }
    };
    let mut ys = vec![head; n];
    for i in 1..n {
        ys[i] = ys[i - 1] + rh + gap_of(i - 1);
    }
    let w = cycles as f64 * cw;
    let bottom = ys[n - 1] + rh;
    let grid = Pen {
        color: GRID,
        width: cv.sw * 0.6,
        dash: false,
    };
    for c in 0..=cycles {
        let x = c as f64 * cw;
        cv.line(p(x, 0.0), p(x, bottom), grid);
    }
    let pen = cv.pen();
    cv.line(p(0.0, head - em * 0.1), p(w, head - em * 0.1), pen);
    for c in 0..cycles {
        cv.label(p(cx(c), head / 2.0 - em * 0.05), &(c + 1).to_string(), INK);
    }
    for (i, r) in rows.iter().enumerate() {
        let y = ys[i];
        row_title(cv, &Some(r.name.clone()), y + rh / 2.0);
        for (c, e) in r.cells.iter().enumerate() {
            if e.is_blank() {
                continue;
            }
            let x = c as f64 * cw;
            let (fill, _, tc) = paint(e.attrs.color);
            let pen = cell_pen(cv, &e.attrs);
            if is_stall(e) {
                cv.rect(x, y, cw, rh, 0.0, Some(fill.unwrap_or(STALL)), Some(pen));
                let h = Pen {
                    color: MUTED,
                    width: cv.sw * 0.6,
                    dash: false,
                };
                hatch(cv, x, y, cw, rh, em * 0.6, false, h);
                cv.rect(x, y, cw, rh, 0.0, None, Some(pen));
            } else {
                cv.rect(x, y, cw, rh, 0.0, fill, Some(pen));
                cv.label(p(x + cw / 2.0, y + rh / 2.0), e.label(), tc);
            }
        }
    }
    for (f, r) in fwds.iter().zip(&routes) {
        let lane_y = |(g, l): (usize, usize)| ys[g] + rh + em * (0.35 + 0.3 * l as f64);
        let (ya, yb) = match r.down {
            1 => (ys[f.from.0] + rh, ys[f.to.0]),
            -1 => (ys[f.from.0], ys[f.to.0] + rh),
            _ => (ys[f.from.0] + rh, ys[f.to.0] + rh),
        };
        let (y1, y2) = (lane_y(r.l1), lane_y(r.l2));
        let mut pts = vec![p(r.sx, ya), p(r.sx, y1)];
        if let Some(xb) = r.xb {
            pts.push(p(xb, y1));
            pts.push(p(xb, y2));
        }
        pts.push(p(r.tx, y2));
        pts.push(p(r.tx, yb));
        let pts = clean(&pts);
        let pen = line_pen(cv, &f.attrs, 1.4);
        cv.arrow_poly(&pts, pen);
    }
    Ok(())
}

// ---------------- gantt ----------------

struct Span {
    a: f64,
    b: f64,
    text: String,
    attrs: Attrs,
}

fn parse_span(mut text: String, mut attrs: Attrs, no: usize) -> Res<Span> {
    let mut label = String::new();
    if let Some(i) = text.find([':', '：']) {
        let c = text[i..].chars().next().unwrap();
        label = text[i + c.len_utf8()..].to_string();
        text.truncate(i);
    }
    let (a, b) = text.split_once('-').ok_or_else(|| {
        fail(
            no,
            format!("interval `{}` should be written start-end", text),
        )
    })?;
    let (a, b) = (
        parse_num(a, no, "interval start")?,
        parse_num(b, no, "interval end")?,
    );
    if b <= a {
        return err(no, format!("interval {}-{} must end after it starts", a, b));
    }
    Ok(Span {
        a,
        b,
        text: label,
        attrs: std::mem::take(&mut attrs),
    })
}

fn parse_spans(l: &Line, toks: &[Tok]) -> Res<Vec<Span>> {
    let mut out: Vec<Span> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            Tok::E(e) if !e.quoted => {
                // `0-3:"就绪 队列"` 被切成两个记号，这里接回去。
                if e.text.ends_with([':', '：']) {
                    if let Some(Tok::E(q)) = toks.get(i + 1) {
                        if q.quoted {
                            let mut a = e.attrs.clone();
                            a.merge(q.attrs.clone());
                            out.push(parse_span(format!("{}{}", e.text, q.text), a, l.no)?);
                            i += 2;
                            continue;
                        }
                    }
                }
                // `3-7:"就绪 队列"` 引号贴着词，空格处被切开了，也接回去。
                let (mut text, mut attrs) = (e.text.clone(), e.attrs.clone());
                if let Some(q) = text.find(":\"").or_else(|| text.find("：\"")) {
                    while text[q..].matches('"').count() < 2 {
                        i += 1;
                        match toks.get(i) {
                            Some(Tok::E(n)) => {
                                text.push(' ');
                                text.push_str(&n.text);
                                attrs.merge(n.attrs.clone());
                            }
                            _ => return err(l.no, "unclosed quote in interval text"),
                        }
                    }
                    text = text.replace('"', "");
                }
                out.push(parse_span(text, attrs, l.no)?);
            }
            Tok::Attrs(a) => match out.last_mut() {
                Some(s) => s.attrs.merge(a.clone()),
                None => return err(l.no, "attributes `[...]` must follow an interval"),
            },
            _ => {
                return err(
                    l.no,
                    "an interval is written `start-end`, `start-end:text` or `start-end[attrs]`",
                )
            }
        }
        i += 1;
    }
    Ok(out)
}

pub fn gantt(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["axis"])?;
    let axis = doc.opt_bool("axis", true)?;
    let mut rows = Vec::new();
    for l in &doc.lines {
        let (mut title, mut toks) = row_head(l);
        // 没有行名的写法 `0-3:P1 3-7:P2`：首词的冒号属于区间。
        if title.as_ref().is_some_and(|t| {
            t.split_once('-')
                .is_some_and(|(a, b)| a.parse::<f64>().is_ok() && b.parse::<f64>().is_ok())
        }) {
            title = None;
            toks = l.toks.clone();
        }
        let spans = parse_spans(l, &toks)?;
        rows.push((title, spans));
    }
    let mut moments: Vec<f64> = rows
        .iter()
        .flat_map(|(_, s)| s.iter().flat_map(|x| [x.a, x.b]))
        .collect();
    if moments.is_empty() {
        return err(doc.head_line, "nothing to draw");
    }
    moments.sort_by(|a, b| a.partial_cmp(b).unwrap());
    moments.dedup();
    let (t0, t1) = (moments[0], *moments.last().unwrap());
    let (em, small) = (cv.em, 0.85);
    let total = t1 - t0;
    let fit_all = em * 40.0 / total;
    // 比例尺：先照顾区间内文字（硬约束），再尽量让刻度不挤，挤不下就错行。
    let mut u = fit_all.min(em * 1.8) * cv.sp.max(0.5);
    for s in rows.iter().flat_map(|(_, s)| s) {
        if !s.text.is_empty() {
            u = u.max((cv.w(&s.text) + em * 0.6) / (s.b - s.a));
        }
    }
    let labels: Vec<String> = moments.iter().map(|t| fmt_num(*t)).collect();
    let mut want = 0.0_f64;
    for i in 1..moments.len() {
        let need = (cv.ws(&labels[i - 1], small) + cv.ws(&labels[i], small)) / 2.0 + em * 0.4;
        want = want.max(need / (moments[i] - moments[i - 1]));
    }
    u = u.max(want.min(fit_all));

    let (rh, gap) = (em * 1.7, em * 0.5 * cv.sp.max(0.5));
    let nr = rows.len() as f64;
    let bottom = nr * (rh + gap);
    let x_of = |t: f64| (t - t0) * u;
    if axis {
        let g = Pen {
            color: GRID,
            width: cv.sw * 0.6,
            dash: true,
        };
        for t in &moments {
            cv.line(p(x_of(*t), 0.0), p(x_of(*t), bottom), g);
        }
    }
    for (i, (title, spans)) in rows.iter().enumerate() {
        let y = i as f64 * (rh + gap);
        row_title(cv, title, y + rh / 2.0);
        for s in spans {
            let (x, w) = (x_of(s.a), (s.b - s.a) * u);
            let (fill, _, tc) = paint(s.attrs.color);
            let pen = cell_pen(cv, &s.attrs);
            cv.rect(x, y, w, rh, 0.0, Some(fill.unwrap_or("#ffffff")), Some(pen));
            cv.label(p(x + w / 2.0, y + rh / 2.0), &s.text, tc);
        }
    }
    if axis {
        let pen = cv.pen();
        cv.line(p(0.0, bottom), p(x_of(t1), bottom), pen);
        let tick = em * 0.3;
        let line_h = em * 1.05;
        let mut ends: Vec<f64> = Vec::new();
        for (t, s) in moments.iter().zip(&labels) {
            let (x, w) = (x_of(*t), cv.ws(s, small));
            let row = ends
                .iter()
                .position(|e| x - w / 2.0 > e + em * 0.25)
                .unwrap_or(ends.len());
            if row == ends.len() {
                ends.push(0.0);
            }
            ends[row] = x + w / 2.0;
            let ty = bottom + tick + em * 0.5 + row as f64 * line_h;
            cv.line(p(x, bottom), p(x, bottom + tick + row as f64 * line_h), pen);
            cv.text(p(x, ty), s, Anchor::Middle, small, INK);
        }
    }
    Ok(())
}

// ---------------- wave ----------------

#[derive(Clone, PartialEq)]
enum Lv {
    Low,
    High,
    Unknown,
    HiZ,
    Bus(String),
}

#[derive(Clone)]
struct Seg {
    a: f64,
    b: f64,
    lv: Lv,
    attrs: Attrs,
}

fn push_seg(v: &mut Vec<Seg>, a: f64, b: f64, lv: Lv, attrs: &Attrs) {
    if let Some(l) = v.last_mut() {
        if l.lv == lv && !matches!(lv, Lv::Bus(_)) && l.attrs.color == attrs.color {
            l.b = b;
            return;
        }
    }
    v.push(Seg {
        a,
        b,
        lv,
        attrs: attrs.clone(),
    });
}

/// 解析一行波形，返回线段和周期数。
fn parse_wave(l: &Line, toks: &[Tok]) -> Res<(Vec<Seg>, usize)> {
    let mut segs: Vec<Seg> = Vec::new();
    let mut c = 0usize;
    let mut i = 0;
    while i < toks.len() {
        let e = match &toks[i] {
            Tok::E(e) => e,
            Tok::Attrs(a) => {
                match segs.last_mut() {
                    Some(s) => s.attrs.merge(a.clone()),
                    None => return err(l.no, "attributes `[...]` must follow a cycle"),
                }
                i += 1;
                continue;
            }
            _ => {
                return err(
                    l.no,
                    "cycles are level characters 0 1 x z, c/clk, =text or `.`",
                )
            }
        };
        i += 1;
        let (x0, x1) = (c as f64, (c + 1) as f64);
        c += 1;
        if e.quoted {
            push_seg(&mut segs, x0, x1, Lv::Bus(e.text.clone()), &e.attrs);
        } else if e.text == "." {
            match segs.last_mut() {
                Some(s) => s.b = x1,
                None => return err(l.no, "`.` continues the previous cycle, but there is none"),
            }
        } else if let Some(s) = e.text.strip_prefix('=') {
            let mut text = s.to_string();
            let mut attrs = e.attrs.clone();
            // `="a b"`：词法层按空格切开了，这里接回去。
            if let Some(rest) = text.strip_prefix('"') {
                let mut t = rest.to_string();
                while !t.ends_with('"') {
                    match toks.get(i) {
                        Some(Tok::E(n)) => {
                            t.push(' ');
                            t.push_str(&n.text);
                            attrs.merge(n.attrs.clone());
                            i += 1;
                        }
                        _ => return err(l.no, "unclosed quote in `=\"...\"`"),
                    }
                }
                t.pop();
                text = t;
            }
            if text.is_empty() {
                return err(l.no, "a bus segment needs text, e.g. `=A1`");
            }
            push_seg(&mut segs, x0, x1, Lv::Bus(text), &attrs);
        } else if e.text == "c" || e.text == "clk" {
            let m = x0 + 0.5;
            push_seg(&mut segs, x0, m, Lv::High, &e.attrs);
            push_seg(&mut segs, m, x1, Lv::Low, &e.attrs);
        } else {
            let cs: Vec<char> = e.text.chars().collect();
            let d = 1.0 / cs.len() as f64;
            for (k, ch) in cs.iter().enumerate() {
                let lv = match ch {
                    '0' => Lv::Low,
                    '1' => Lv::High,
                    'x' | 'X' => Lv::Unknown,
                    'z' | 'Z' => Lv::HiZ,
                    _ => {
                        return err(
                            l.no,
                            format!(
                                "unknown cycle `{}` (available: 0 1 x z, c, clk, =text, .)",
                                e.text
                            ),
                        )
                    }
                };
                let a = x0 + k as f64 * d;
                push_seg(&mut segs, a, a + d, lv, &e.attrs);
            }
        }
    }
    Ok((segs, c))
}

pub fn wave(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["grid", "bits"])?;
    let grid = doc.opt_bool("grid", false)?;
    let bits: Vec<String> = doc
        .opt("bits")
        .map(|s| s.split_whitespace().map(|x| x.to_string()).collect())
        .unwrap_or_default();
    let mut rows = Vec::new();
    let mut n = bits.len();
    for l in &doc.lines {
        let (title, toks) = row_head(l);
        if title.is_none() {
            return err(l.no, "a signal is written `name: cycles...`");
        }
        let (segs, c) = parse_wave(l, &toks)?;
        n = n.max(c);
        rows.push((title, segs));
    }
    if n == 0 {
        return err(doc.head_line, "nothing to draw");
    }
    let em = cv.em;
    let mut cw = em * 3.2;
    let slant_of = |w: f64| (em * 0.4).min(w / 4.0);
    for (_, segs) in &rows {
        for s in segs {
            let d = s.b - s.a;
            if let Lv::Bus(t) = &s.lv {
                cw = cw.max((cv.w(t) + em * 1.4) / d);
            } else if d > 0.0 && d < 1.0 {
                cw = cw.max(em * 0.9 / d);
            }
        }
    }
    for b in &bits {
        cw = cw.max(cv.w(b) + em * 0.6);
    }
    cw *= cv.sp.max(0.5);
    let h = em * 1.6;
    let gap = em * 0.8 * cv.sp.max(0.5);
    let head = if bits.is_empty() { 0.0 } else { em * 1.5 };
    let bottom = head + rows.len() as f64 * (h + gap) - gap;
    if grid {
        let g = Pen {
            color: GRID,
            width: cv.sw * 0.6,
            dash: true,
        };
        for c in 0..=n {
            let x = c as f64 * cw;
            cv.line(p(x, -em * 0.1), p(x, bottom + em * 0.2), g);
        }
    }
    for (c, b) in bits.iter().enumerate() {
        cv.label(p((c as f64 + 0.5) * cw, em * 0.5), b, INK);
    }
    for (i, (title, segs)) in rows.iter().enumerate() {
        let top = head + i as f64 * (h + gap);
        let (bot, mid) = (top + h, top + h / 2.0);
        row_title(cv, title, mid);
        let level = |lv: &Lv| match lv {
            Lv::High => Some(top),
            Lv::Low => Some(bot),
            Lv::HiZ => Some(mid),
            Lv::Bus(_) => Some(mid),
            Lv::Unknown => None,
        };
        for (k, s) in segs.iter().enumerate() {
            let (x0, x1) = (s.a * cw, s.b * cw);
            let pen = line_pen(cv, &s.attrs, 1.3);
            match &s.lv {
                Lv::Bus(t) => {
                    let sl = slant_of(x1 - x0);
                    let pts = [
                        p(x0, mid),
                        p(x0 + sl, top),
                        p(x1 - sl, top),
                        p(x1, mid),
                        p(x1 - sl, bot),
                        p(x0 + sl, bot),
                    ];
                    let (fill, _, tc) = paint(s.attrs.color);
                    polygon(cv, &pts, Some(fill.unwrap_or("#ffffff")), Some(pen));
                    cv.label(p((x0 + x1) / 2.0, mid), t, tc);
                }
                Lv::Unknown => {
                    let hp = Pen {
                        color: MUTED,
                        width: cv.sw * 0.6,
                        dash: false,
                    };
                    hatch(cv, x0, top, x1 - x0, h, em * 0.5, true, hp);
                    cv.rect(x0, top, x1 - x0, h, 0.0, None, Some(pen));
                }
                lv => {
                    let y = level(lv).unwrap();
                    cv.line(p(x0, y), p(x1, y), pen);
                }
            }
            // 与前一段的衔接竖线；不定态的方框自带竖边。
            if k > 0 && (segs[k - 1].b - s.a).abs() < 1e-9 {
                if let (Some(y0), Some(y1)) = (level(&segs[k - 1].lv), level(&s.lv)) {
                    if (y0 - y1).abs() > 1e-9 {
                        cv.line(p(x0, y0), p(x0, y1), pen);
                    }
                }
            }
        }
    }
    Ok(())
}

// ---------------- ladder ----------------

struct Msg {
    from: usize,
    to: usize,
    row: f64,
    lost: bool,
    thick: bool,
    text: String,
    attrs: Attrs,
}

struct Note {
    actor: usize,
    row: f64,
    text: String,
    attrs: Attrs,
}

/// 消息箭头：a 到 b，丢失时 b 已是半路的终点。
struct MsgGeom {
    a: P,
    b: P,
}

pub fn ladder(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["actors", "slope"])?;
    let actors = doc
        .opt_list("actors")
        .ok_or_else(|| fail(doc.head_line, "ladder needs actors=A,B"))?;
    if !(2..=5).contains(&actors.len()) {
        return err(
            doc.head_line,
            format!("ladder takes 2 to 5 actors, got {}", actors.len()),
        );
    }
    let slope = match doc.opt("slope") {
        Some(s) => parse_num(s, doc.head_line, "slope")?,
        None => 1.0,
    };
    if slope < 0.0 {
        return err(doc.head_line, "slope must not be negative");
    }
    let find = |name: &str, no: usize| -> Res<usize> {
        actors.iter().position(|a| a == name).ok_or_else(|| {
            fail(
                no,
                format!("unknown actor `{}` (available: {})", name, actors.join(" ")),
            )
        })
    };
    let mut msgs: Vec<Msg> = Vec::new();
    let mut notes: Vec<Note> = Vec::new();
    let mut cursor = 1.0_f64;
    // 各角色最近一次被消息碰到的行，note 默认标在这一行。
    let mut touch: Vec<Option<f64>> = vec![None; actors.len()];
    for l in &doc.lines {
        let t = &l.toks;
        let first = match t.first() {
            Some(Tok::E(e)) => e,
            _ => return err(l.no, "expected a message, `note` or `gap`"),
        };
        let op = match t.get(1) {
            Some(Tok::Arrow) => Some((false, false)),
            Some(Tok::E(e)) if e.is("=>") => Some((false, true)),
            Some(Tok::E(e)) if e.is("-x") => Some((true, false)),
            _ => None,
        };
        // 消息与 note 共用的尾部：文字、@行号、属性。
        let tail = |toks: &[Tok], attrs: &mut Attrs| -> Res<(String, Option<f64>)> {
            let (mut text, mut at) = (None, None);
            for x in toks {
                match x {
                    Tok::E(e) if text.is_none() => {
                        text = Some(e.text.clone());
                        attrs.merge(e.attrs.clone());
                    }
                    Tok::At(s) => at = Some(parse_num(s, l.no, "row")?),
                    Tok::Attrs(a) => attrs.merge(a.clone()),
                    _ => {
                        return err(
                            l.no,
                            "unexpected trailing content; expected `\"text\" @row [attrs]`",
                        )
                    }
                }
            }
            if at.is_some_and(|r| r < 1.0) {
                return err(l.no, "rows are numbered from 1");
            }
            Ok((text.unwrap_or_default(), at))
        };
        if op.is_none() && first.is("gap") {
            if t.len() > 1 {
                return err(l.no, "`gap` takes no arguments");
            }
            cursor += 1.0;
            touch.iter_mut().for_each(|x| *x = None);
        } else if op.is_none() && first.is("note") {
            let name = match t.get(1) {
                Some(Tok::E(e)) => e,
                _ => return err(l.no, "a note is written `note actor \"text\"`"),
            };
            let actor = find(&name.text, l.no)?;
            let mut attrs = name.attrs.clone();
            let (text, at) = tail(&t[2..], &mut attrs)?;
            let row = at.or(touch[actor]).unwrap_or(cursor);
            notes.push(Note {
                actor,
                row,
                text,
                attrs,
            });
        } else if let Some((lost, thick)) = op {
            let to = match t.get(2) {
                Some(Tok::E(e)) => e,
                _ => return err(l.no, "a message is written `A -> B \"text\"`"),
            };
            let (from_i, to_i) = (find(&first.text, l.no)?, find(&to.text, l.no)?);
            if from_i == to_i {
                return err(l.no, "a message needs two different actors");
            }
            let mut attrs = first.attrs.clone();
            attrs.merge(to.attrs.clone());
            if let Tok::E(e) = &t[1] {
                attrs.merge(e.attrs.clone());
            }
            let (text, at) = tail(&t[3..], &mut attrs)?;
            let row = at.unwrap_or(cursor);
            cursor = row + 1.0;
            touch[from_i] = Some(row);
            if !lost {
                touch[to_i] = Some(row + slope);
            }
            msgs.push(Msg {
                from: from_i,
                to: to_i,
                row,
                lost,
                thick,
                text,
                attrs,
            });
        } else {
            return err(
                l.no,
                "expected `A -> B`, `A => B`, `A -x B`, `note` or `gap`",
            );
        }
    }
    if msgs.is_empty() && notes.is_empty() {
        return err(doc.head_line, "nothing to draw");
    }

    let em = cv.em;
    let sp = cv.sp.max(0.5);
    let mut step = em * 14.0;
    for m in &msgs {
        let span = m.from.abs_diff(m.to) as f64;
        let w = cv.ws(&m.text, 0.85);
        let need = if m.lost {
            (w + em * 2.5) * 2.0
        } else {
            w + em * 9.0
        };
        step = step.max(need / span);
    }
    step = step.max(
        actors
            .iter()
            .map(|a| cv.w(a) + em * 2.0)
            .fold(0.0, f64::max),
    );
    step *= sp;
    let rowh = em * 2.6 * sp;
    let x_of = |i: usize| i as f64 * step;
    let y_of = |r: f64| r * rowh;
    let last = msgs
        .iter()
        .map(|m| m.row + slope)
        .chain(notes.iter().map(|n| n.row))
        .fold(1.0, f64::max);
    let end = y_of(last) + em * 1.2;

    let box_h = em * 1.8;
    let ytop = y_of(1.0) - em * 1.5;
    let pen = cv.pen();
    for (i, a) in actors.iter().enumerate() {
        let w = cv.w(a) + em * 1.2;
        let x = x_of(i);
        cv.line(p(x, ytop), p(x, end), pen);
        cv.rect(x - w / 2.0, ytop - box_h, w, box_h, 0.0, None, Some(pen));
        cv.label(p(x, ytop - box_h / 2.0), a, INK);
    }

    let geom: Vec<MsgGeom> = msgs
        .iter()
        .map(|m| {
            let a = p(x_of(m.from), y_of(m.row));
            let b = p(x_of(m.to), y_of(m.row + slope));
            MsgGeom {
                a,
                b: if m.lost { a.mid(b) } else { b },
            }
        })
        .collect();
    for (m, g) in msgs.iter().zip(&geom) {
        let pen = line_pen(cv, &m.attrs, if m.thick { 2.0 } else { 1.0 });
        if m.lost {
            cv.line(g.a, g.b, pen);
            let r = em * 0.42;
            let (u, v) = (p(r, r), p(r, -r));
            let x = line_pen(cv, &m.attrs, 1.8);
            cv.line(g.b.sub(u), g.b.add(u), x);
            cv.line(g.b.sub(v), g.b.add(v), x);
        } else {
            cv.arrow(g.a, g.b, pen);
        }
    }

    // 消息文字压在自己的箭头上，半透明底不断开线条；放不下时由避让挪开。
    for (m, g) in msgs.iter().zip(&geom) {
        if !m.text.is_empty() {
            cv.soft_label(Soft {
                text: m.text.clone(),
                scale: 0.85,
                color: paint(m.attrs.color).2.to_string(),
                path: vec![g.a, g.b],
                ts: SENDER_TS.to_vec(),
                side: p(0.0, -1.0),
                fixed: None,
                inline_first: true,
            });
        }
    }

    // 状态标注放生命线外侧；中间角色放右侧并加白底。
    let mut seen: Vec<(usize, f64, usize)> = Vec::new();
    for n in &notes {
        let k = match seen.iter_mut().find(|s| s.0 == n.actor && s.1 == n.row) {
            Some(s) => {
                s.2 += 1;
                s.2
            }
            None => {
                seen.push((n.actor, n.row, 0));
                0
            }
        };
        let y = y_of(n.row) + k as f64 * em * 1.15;
        let w = cv.ws(&n.text, 0.85);
        let left = n.actor == 0;
        let x = x_of(n.actor) + if left { -em * 0.5 } else { em * 0.5 };
        let (fill, _, tc) = paint(n.attrs.color);
        let bg = fill.or(if n.actor == 0 || n.actor == actors.len() - 1 {
            None
        } else {
            Some("#ffffff")
        });
        if bg.is_some() {
            let x0 = if left { x - w } else { x };
            let pad = em * 0.15;
            cv.rect(
                x0 - pad,
                y - em * 0.55,
                w + pad * 2.0,
                em * 1.1,
                0.0,
                bg,
                None,
            );
        }
        let anchor = if left { Anchor::End } else { Anchor::Start };
        cv.text(p(x, y), &n.text, anchor, 0.85, tc);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_total_and_minimum() {
        let w = fit(&[1.0, 8.0, 23.0], &[20.0, 10.0, 10.0], 320.0);
        assert!((w.iter().sum::<f64>() - 320.0).abs() < 1e-6);
        assert!((w[0] - 20.0).abs() < 1e-9);
        assert!((w[2] / w[1] - 23.0 / 8.0).abs() < 1e-9);
    }

    #[test]
    fn clean_drops_collinear() {
        let v = clean(&[
            p(0.0, 0.0),
            p(0.0, 5.0),
            p(0.0, 5.0),
            p(0.0, 9.0),
            p(3.0, 9.0),
        ]);
        assert_eq!(v.len(), 3);
    }
}
