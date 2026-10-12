//! list / array / stack / ring

use crate::parse::{err, Doc, Elem, Res, Tok};
use crate::svg::{p, paint, Anchor, Canvas, Pen, INK, MUTED, P};

/// 在指针域里画 ∧（空指针）。
pub fn null_mark(cv: &mut Canvas, c: P, w: f64, h: f64) {
    let pen = cv.pen();
    cv.polyline(
        &[
            p(c.x - w * 0.3, c.y + h * 0.28),
            p(c.x, c.y - h * 0.28),
            p(c.x + w * 0.3, c.y + h * 0.28),
        ],
        pen,
    );
}

/// 画一个格子：底色、边框、文字。
pub fn cell(cv: &mut Canvas, x: f64, y: f64, w: f64, h: f64, e: &Elem) {
    let (fill, _, tc) = paint(e.attrs.color);
    let pen = Pen {
        color: INK,
        width: cv.sw,
        dash: e.attrs.dashed,
    };
    cv.rect(x, y, w, h, 0.0, fill, Some(pen));
    cv.label(p(x + w / 2.0, y + h / 2.0), e.label(), tc);
}

// ---------------- list ----------------

enum Tail {
    None,
    Null,
    Back(usize),
}

struct LRow {
    label: Option<String>,
    items: Vec<Elem>,
    double: bool,
    lead_null: bool,
    tail: Tail,
}

fn parse_list_row(l: &crate::parse::Line) -> Res<LRow> {
    let (label, toks) = l.split_row_label();
    let mut r = LRow {
        label,
        items: Vec::new(),
        double: false,
        lead_null: false,
        tail: Tail::None,
    };
    let mut expect_item = true;
    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            Tok::E(e) if e.is("^") => {
                if r.items.is_empty() && i + 1 < toks.len() {
                    r.lead_null = true;
                } else if i + 1 == toks.len() && !r.items.is_empty() && !expect_item {
                    return err(l.no, "`^` must follow `->`");
                } else if i + 1 == toks.len() && !r.items.is_empty() {
                    r.tail = Tail::Null;
                } else {
                    return err(
                        l.no,
                        "`^` (null pointer) may only appear at the start or end",
                    );
                }
                expect_item = false;
            }
            Tok::E(e) => {
                if !expect_item {
                    return err(l.no, format!("missing `->` before `{}`", e.text));
                }
                r.items.push(e.clone());
                expect_item = false;
            }
            Tok::Arrow | Tok::DArrow => {
                if matches!(toks[i], Tok::DArrow) {
                    r.double = true;
                }
                if expect_item && !r.lead_null {
                    return err(l.no, "missing node before the arrow");
                }
                expect_item = true;
            }
            Tok::At(k) => {
                if i + 1 != toks.len() {
                    return err(l.no, "`@k` may only appear at the end");
                }
                let k: usize = k.parse().map_err(|_| {
                    crate::parse::fail(l.no, format!("`@{}` should be a node index", k))
                })?;
                if k == 0 || k > r.items.len() {
                    return err(
                        l.no,
                        format!(
                            "`@{}` is out of range (this line has {} nodes)",
                            k,
                            r.items.len()
                        ),
                    );
                }
                r.tail = Tail::Back(k - 1);
                expect_item = false;
            }
            _ => {
                return err(
                    l.no,
                    "`-` or a standalone `[...]` is not allowed in a list line",
                )
            }
        }
        i += 1;
    }
    if r.items.is_empty() {
        return err(l.no, "this line has no nodes");
    }
    if expect_item {
        return err(
            l.no,
            "the trailing arrow needs a node, `^` or `@k` after it",
        );
    }
    Ok(r)
}

pub fn list(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&[])?;
    let rows: Vec<LRow> = doc.lines.iter().map(parse_list_row).collect::<Res<_>>()?;
    if rows.is_empty() {
        return err(doc.head_line, "the list is empty");
    }
    let em = cv.em;
    let h = em * 1.6;
    let pw = em * 1.0;
    let gap = em * 1.8 * cv.sp;
    let ncol = rows.iter().map(|r| r.items.len()).max().unwrap();
    let dw = |cv: &Canvas, e: &Elem| (cv.w(e.label()) + em * 0.8).max(em * 1.8);
    let mut colw = vec![0.0f64; ncol];
    for r in &rows {
        for (j, e) in r.items.iter().enumerate() {
            let tot = dw(cv, e) + pw * if r.double { 2.0 } else { 1.0 };
            colw[j] = colw[j].max(tot);
        }
    }
    let mut y = 0.0;
    for r in &rows {
        let has_ptr = r.items.iter().any(|e| !e.attrs.ptrs.is_empty());
        if has_ptr {
            y += em * 2.6;
        }
        if r.double && matches!(r.tail, Tail::Back(_)) {
            y += em * 1.0;
        }
        let cy = y + h / 2.0;
        if let Some(l) = &r.label {
            cv.text(p(-em * 0.5, cy), l, Anchor::End, 1.0, INK);
        }
        // 各结点位置
        let mut xs = Vec::new();
        let mut x = 0.0;
        for (j, e) in r.items.iter().enumerate() {
            xs.push((x, dw(cv, e)));
            x += colw[j] + gap;
        }
        let tw = |j: usize| xs[j].1 + pw * if r.double { 2.0 } else { 1.0 };
        for (j, e) in r.items.iter().enumerate() {
            let (x0, w) = xs[j];
            let mut x = x0;
            let pen = Pen {
                color: INK,
                width: cv.sw,
                dash: e.attrs.dashed,
            };
            if r.double {
                cv.rect(x, y, pw, h, 0.0, None, Some(pen));
                x += pw;
            }
            let (fill, _, tc) = paint(e.attrs.color);
            cv.rect(x, y, w, h, 0.0, fill, Some(pen));
            cv.label(p(x + w / 2.0, cy), e.label(), tc);
            if !e.attrs.ptrs.is_empty() {
                cv.pointer(p(x + w / 2.0, y), p(0.0, -1.0), &e.attrs.ptrs);
            }
            x += w;
            cv.rect(x, y, pw, h, 0.0, None, Some(pen));
        }
        let pen = cv.pen();
        let off = if r.double { h * 0.2 } else { 0.0 };
        for j in 0..r.items.len().saturating_sub(1) {
            let (x0, _) = xs[j];
            let fx = x0 + tw(j) - pw / 2.0;
            let nx = xs[j + 1].0;
            cv.arrow(p(fx, cy - off), p(nx, cy - off), pen);
            if r.double {
                let px = nx + pw / 2.0;
                cv.arrow(p(px, cy + off), p(x0 + tw(j), cy + off), pen);
            }
        }
        let last = r.items.len() - 1;
        let lfx = xs[last].0 + tw(last) - pw / 2.0;
        if r.lead_null && r.double {
            null_mark(cv, p(xs[0].0 + pw / 2.0, cy), pw, h);
        }
        let mut extra = 0.0;
        match r.tail {
            Tail::None => {}
            Tail::Null => null_mark(cv, p(lfx, cy), pw, h),
            Tail::Back(k) => {
                let bot = y + h;
                let low = bot + em * 1.0;
                let kx = xs[k].0 + tw(k) / 2.0;
                cv.arrow_poly(&[p(lfx, cy), p(lfx, low), p(kx, low), p(kx, bot)], pen);
                extra = em * 1.2;
                if r.double {
                    let top = y;
                    let high = top - em * 0.9;
                    let kpx = xs[k].0 + pw / 2.0;
                    let lx = xs[last].0 + tw(last) / 2.0;
                    cv.arrow_poly(&[p(kpx, cy), p(kpx, high), p(lx, high), p(lx, top)], pen);
                }
            }
        }
        y += h + extra + em * 1.2 * cv.sp;
    }
    Ok(())
}

// ---------------- array ----------------

pub fn array(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["index"])?;
    let index = doc.opt_num("index")?;
    if doc.lines.is_empty() {
        return err(doc.head_line, "the array is empty");
    }
    let em = cv.em;
    let rows: Vec<(Option<String>, Vec<Elem>)> = doc
        .lines
        .iter()
        .map(|l| {
            let (lab, toks) = l.split_row_label();
            let es = crate::parse::Line {
                no: l.no,
                indent: 0,
                toks,
            }
            .elems()?;
            Ok((lab, es))
        })
        .collect::<Res<_>>()?;
    let maxw = rows
        .iter()
        .flat_map(|r| r.1.iter())
        .map(|e| cv.w(e.label()))
        .fold(0.0, f64::max);
    let cw = (maxw + em * 0.8).max(em * 1.8);
    let h = em * 1.7;
    let mut y = 0.0;
    let n = rows.len();
    for (ri, (lab, es)) in rows.iter().enumerate() {
        if es.iter().any(|e| !e.attrs.ptrs.is_empty()) {
            y += em * 2.6;
        }
        if let Some(l) = lab {
            cv.text(p(-em * 0.5, y + h / 2.0), l, Anchor::End, 1.0, INK);
        }
        for (j, e) in es.iter().enumerate() {
            let x = j as f64 * cw;
            cell(cv, x, y, cw, h, e);
            if !e.attrs.ptrs.is_empty() {
                cv.pointer(p(x + cw / 2.0, y), p(0.0, -1.0), &e.attrs.ptrs);
            }
        }
        y += h;
        if ri + 1 == n {
            if let Some(start) = index {
                for j in 0..es.len() {
                    let s = (start + j as i64).to_string();
                    cv.text(
                        p(j as f64 * cw + cw / 2.0, y + em * 0.65),
                        &s,
                        Anchor::Middle,
                        0.75,
                        MUTED,
                    );
                }
            }
        }
        y += em * 0.6 * cv.sp;
    }
    Ok(())
}

// ---------------- stack ----------------

pub fn stack(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["index", "size"])?;
    let index = doc.opt_num("index")?;
    let size = doc.opt_num("size")?.unwrap_or(0).max(0) as usize;
    if doc.lines.is_empty() {
        return err(doc.head_line, "the stack is empty");
    }
    let em = cv.em;
    let h = em * 1.6;
    let mut x = 0.0;
    for l in &doc.lines {
        let (lab, toks) = l.split_row_label();
        let mut es = crate::parse::Line {
            no: l.no,
            indent: 0,
            toks,
        }
        .elems()?;
        if size > 0 && es.len() > size {
            return err(l.no, format!("{} elements exceed size={}", es.len(), size));
        }
        while es.len() < size {
            es.push(Elem {
                text: "_".into(),
                quoted: false,
                attrs: Default::default(),
            });
        }
        let maxw = es.iter().map(|e| cv.w(e.label())).fold(0.0, f64::max);
        let cw = (maxw + em * 1.2).max(em * 2.6);
        let n = es.len();
        for (i, e) in es.iter().enumerate() {
            let y = (n - 1 - i) as f64 * h;
            cell(cv, x, y, cw, h, e);
            if !e.attrs.ptrs.is_empty() {
                cv.pointer(p(x + cw, y + h / 2.0), p(1.0, 0.0), &e.attrs.ptrs);
            }
            if let Some(s) = index {
                cv.text(
                    p(x - em * 0.4, y + h / 2.0),
                    &(s + i as i64).to_string(),
                    Anchor::End,
                    0.75,
                    MUTED,
                );
            }
        }
        if let Some(lab) = lab {
            cv.label(p(x + cw / 2.0, n as f64 * h + em * 0.9), &lab, INK);
        }
        let ptr_w = es
            .iter()
            .filter(|e| !e.attrs.ptrs.is_empty())
            .map(|e| cv.w(&e.attrs.ptrs.join(", ")) + em * 2.4)
            .fold(0.0, f64::max);
        x += cw + ptr_w + em * 2.0 * cv.sp;
    }
    Ok(())
}

// ---------------- ring ----------------

pub fn ring(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["size", "index"])?;
    let index = doc.opt_bool("index", true)?;
    let mut es: Vec<Elem> = Vec::new();
    for l in &doc.lines {
        es.extend(l.elems()?);
    }
    let size = doc
        .opt_num("size")?
        .map(|s| s.max(0) as usize)
        .unwrap_or(es.len());
    if es.len() > size {
        return err(
            doc.first_line(),
            format!("{} elements exceed size={}", es.len(), size),
        );
    }
    if size < 2 {
        return err(
            doc.head_line,
            "a ring buffer needs at least 2 slots (set them with size=)",
        );
    }
    while es.len() < size {
        es.push(Elem {
            text: "_".into(),
            quoted: false,
            attrs: Default::default(),
        });
    }
    let em = cv.em;
    let n = size as f64;
    let maxw = es.iter().map(|e| cv.w(e.label())).fold(0.0, f64::max);
    let tau = std::f64::consts::TAU;
    let rm = (em * 3.4)
        .max((maxw + em * 0.9) * n / tau)
        .max(em * 2.0 * n / tau)
        * cv.sp.max(0.5);
    let (ri, ro) = (rm - em * 1.1, rm + em * 1.1);
    let at = |a: f64, r: f64| p(a.cos() * r, a.sin() * r);
    let step = tau / n;
    for (i, e) in es.iter().enumerate() {
        let c = -tau / 4.0 + i as f64 * step;
        let (a0, a1) = (c - step / 2.0, c + step / 2.0);
        let (p0, p1, p2, p3) = (at(a0, ro), at(a1, ro), at(a1, ri), at(a0, ri));
        let d = format!(
            "M{:.2} {:.2} A{:.2} {:.2} 0 0 1 {:.2} {:.2} L{:.2} {:.2} A{:.2} {:.2} 0 0 0 {:.2} {:.2} Z",
            p0.x, p0.y, ro, ro, p1.x, p1.y, p2.x, p2.y, ri, ri, p3.x, p3.y
        );
        let (fill, _, tc) = paint(e.attrs.color);
        let pen = Pen {
            color: INK,
            width: cv.sw,
            dash: e.attrs.dashed,
        };
        cv.path(&d, fill, Some(pen), &[p0, p1, p2, p3, at(c, ro)]);
        cv.label(at(c, rm), e.label(), tc);
        if index {
            cv.text(
                at(c, ri - em * 0.6),
                &i.to_string(),
                Anchor::Middle,
                0.7,
                MUTED,
            );
        }
        if !e.attrs.ptrs.is_empty() {
            cv.pointer(at(c, ro), at(c, 1.0), &e.attrs.ptrs);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn list_rows() {
        let d = parse("list\nL: 头 -> a -> b -> ^\n^ <-> x <-> y @1").unwrap();
        let r = parse_list_row(&d.lines[0]).unwrap();
        assert_eq!(r.label.as_deref(), Some("L"));
        assert_eq!(r.items.len(), 3);
        assert!(matches!(r.tail, Tail::Null));
        let r = parse_list_row(&d.lines[1]).unwrap();
        assert!(r.double && r.lead_null);
        assert!(matches!(r.tail, Tail::Back(0)));
    }

    #[test]
    fn list_errors() {
        for s in [
            "list\na b",
            "list\na -> b @5",
            "list\na ->",
            "list\na -> ^ -> b",
        ] {
            let d = parse(s).unwrap();
            assert!(parse_list_row(&d.lines[0]).is_err(), "{}", s);
        }
    }
}
