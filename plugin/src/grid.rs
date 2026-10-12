//! matrix / table

use crate::parse::{err, Doc, Elem, Res};
use crate::svg::{p, paint, Anchor, Canvas, Pen, INK, MUTED};

fn rows(doc: &Doc) -> Res<Vec<Vec<Elem>>> {
    let r: Vec<Vec<Elem>> = doc.lines.iter().map(|l| l.elems()).collect::<Res<_>>()?;
    if r.is_empty() {
        return err(doc.head_line, "nothing to draw");
    }
    Ok(r)
}

pub fn matrix(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["rows", "cols", "labels"])?;
    let rs = rows(doc)?;
    let em = cv.em;
    let nr = rs.len();
    let nc = rs.iter().map(|r| r.len()).max().unwrap();
    let both = doc.opt_list("labels");
    let rl = doc.opt_list("rows").or_else(|| both.clone());
    let cl = doc.opt_list("cols").or(both);
    for (name, l, n) in [("rows", &rl, nr), ("cols", &cl, nc)] {
        if let Some(l) = l {
            if l.len() != n {
                return err(
                    doc.head_line,
                    format!(
                        "{} has {} labels, but the matrix has {} {}",
                        name,
                        l.len(),
                        n,
                        if name == "rows" { "rows" } else { "columns" }
                    ),
                );
            }
        }
    }
    let maxw = rs
        .iter()
        .flatten()
        .map(|e| cv.w(text(e)))
        .fold(0.0, f64::max);
    let cw = (maxw + em * 0.8).max(em * 1.6) * cv.sp.max(0.5);
    let ch = em * 1.5 * cv.sp.max(0.5);
    let (w, h) = (nc as f64 * cw, nr as f64 * ch);
    for (i, r) in rs.iter().enumerate() {
        for (j, e) in r.iter().enumerate() {
            let (x, y) = (j as f64 * cw, i as f64 * ch);
            let (fill, _, tc) = paint(e.attrs.color);
            if fill.is_some() {
                cv.rect(x, y, cw, ch, 0.0, fill, None);
            }
            cv.label(p(x + cw / 2.0, y + ch / 2.0), text(e), tc);
        }
    }
    let pen = cv.pen();
    let b = em * 0.4;
    let pad = em * 0.15;
    cv.polyline(
        &[
            p(b - pad, -pad),
            p(-pad, -pad),
            p(-pad, h + pad),
            p(b - pad, h + pad),
        ],
        pen,
    );
    cv.polyline(
        &[
            p(w - b + pad, -pad),
            p(w + pad, -pad),
            p(w + pad, h + pad),
            p(w - b + pad, h + pad),
        ],
        pen,
    );
    if let Some(l) = rl {
        for (i, s) in l.iter().enumerate() {
            cv.text(
                p(-em * 0.6, i as f64 * ch + ch / 2.0),
                s,
                Anchor::End,
                0.85,
                MUTED,
            );
        }
    }
    if let Some(l) = cl {
        for (j, s) in l.iter().enumerate() {
            cv.text(
                p(j as f64 * cw + cw / 2.0, -em * 0.8),
                s,
                Anchor::Middle,
                0.85,
                MUTED,
            );
        }
    }
    Ok(())
}

fn text(e: &Elem) -> &str {
    if e.is(".") {
        ""
    } else {
        e.label()
    }
}

pub fn table(doc: &Doc, cv: &mut Canvas) -> Res<()> {
    doc.check_opts(&["head"])?;
    let head = doc.opt_choice("head", &["row", "col", "both", "none"], "none")?;
    let rs = rows(doc)?;
    let em = cv.em;
    let nc = rs.iter().map(|r| r.len()).max().unwrap();
    let mut cw = vec![em * 1.6; nc];
    for r in &rs {
        for (j, e) in r.iter().enumerate() {
            cw[j] = cw[j].max(cv.w(e.label()) + em * 0.8 * cv.sp.max(0.5));
        }
    }
    let ch = em * 1.6;
    for (i, r) in rs.iter().enumerate() {
        let mut x = 0.0;
        let y = i as f64 * ch;
        for j in 0..nc {
            let is_head = (i == 0 && (head == "row" || head == "both"))
                || (j == 0 && (head == "col" || head == "both"));
            let e = r.get(j);
            let color = e.and_then(|e| e.attrs.color);
            let (mut fill, _, tc) = paint(color);
            if fill.is_none() && is_head {
                fill = Some("#eeeeee");
            }
            let dash = e.map_or(false, |e| e.attrs.dashed);
            cv.rect(
                x,
                y,
                cw[j],
                ch,
                0.0,
                fill,
                Some(Pen {
                    color: INK,
                    width: cv.sw,
                    dash,
                }),
            );
            if let Some(e) = e {
                cv.label(p(x + cw[j] / 2.0, y + ch / 2.0), e.label(), tc);
                if !e.attrs.ptrs.is_empty() && i == 0 {
                    cv.pointer(p(x + cw[j] / 2.0, y), p(0.0, -1.0), &e.attrs.ptrs);
                }
            }
            x += cw[j];
        }
    }
    Ok(())
}
