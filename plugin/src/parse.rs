//! 词法与通用语法：首行 `种类 键=值 …`，正文按行切成记号。

pub type Res<T> = Result<T, String>;

pub fn err<T>(line: usize, msg: impl AsRef<str>) -> Res<T> {
    Err(fail(line, msg))
}

pub fn fail(line: usize, msg: impl AsRef<str>) -> String {
    if line == 0 {
        format!("dsviz: {}", msg.as_ref())
    } else {
        format!("dsviz line {}: {}", line, msg.as_ref())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Hl,
    Red,
    Green,
    Blue,
    Gray,
}

#[derive(Clone, Default, Debug)]
pub struct Attrs {
    pub color: Option<Color>,
    pub dashed: bool,
    pub boxed: bool,
    pub ptrs: Vec<String>,
}

impl Attrs {
    fn merge(&mut self, o: Attrs) {
        if o.color.is_some() {
            self.color = o.color;
        }
        self.dashed |= o.dashed;
        self.boxed |= o.boxed;
        self.ptrs.extend(o.ptrs);
    }
}

#[derive(Clone, Debug)]
pub struct Elem {
    pub text: String,
    pub quoted: bool,
    pub attrs: Attrs,
}

impl Elem {
    /// `_`：空位 / 空格子。
    pub fn is_blank(&self) -> bool {
        !self.quoted && self.text == "_"
    }
    pub fn is(&self, s: &str) -> bool {
        !self.quoted && self.text == s
    }
    /// 显示用的文字，空位为空串。
    pub fn label(&self) -> &str {
        if self.is_blank() {
            ""
        } else {
            &self.text
        }
    }
    /// 行标签：未加引号、以冒号结尾的词，如 `L:`、`第1趟:`。
    pub fn row_label(&self) -> Option<String> {
        if self.quoted {
            return None;
        }
        for c in [':', '：'] {
            if let Some(s) = self.text.strip_suffix(c) {
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub enum Tok {
    E(Elem),
    Dash,
    Arrow,
    DArrow,
    Attrs(Attrs),
    At(String),
}

#[derive(Clone, Debug)]
pub struct Line {
    pub no: usize,
    pub indent: usize,
    pub toks: Vec<Tok>,
}

impl Line {
    /// 整行只允许普通元素时取出它们。
    pub fn elems(&self) -> Res<Vec<Elem>> {
        self.toks
            .iter()
            .map(|t| match t {
                Tok::E(e) => Ok(e.clone()),
                _ => err(
                    self.no,
                    "only elements are allowed here, not `-`, `->`, `@` or a standalone `[...]`",
                ),
            })
            .collect()
    }

    /// 去掉开头的行标签。
    pub fn split_row_label(&self) -> (Option<String>, Vec<Tok>) {
        if let Some(Tok::E(e)) = self.toks.first() {
            if let Some(l) = e.row_label() {
                return (Some(l), self.toks[1..].to_vec());
            }
        }
        (None, self.toks.clone())
    }
}

pub struct Doc {
    pub kind: String,
    pub head_line: usize,
    pub opts: Vec<(String, String)>,
    pub lines: Vec<Line>,
}

impl Doc {
    pub fn opt(&self, k: &str) -> Option<&str> {
        self.opts
            .iter()
            .rev()
            .find(|(a, _)| a == k)
            .map(|(_, v)| v.as_str())
    }

    pub fn check_opts(&self, allowed: &[&str]) -> Res<()> {
        for (k, _) in &self.opts {
            if !allowed.contains(&k.as_str()) {
                let list = if allowed.is_empty() {
                    "none".to_string()
                } else {
                    allowed.join(" ")
                };
                return err(
                    self.head_line,
                    format!(
                        "{} does not support option `{}` (available: {})",
                        self.kind, k, list
                    ),
                );
            }
        }
        Ok(())
    }

    pub fn opt_bool(&self, k: &str, def: bool) -> Res<bool> {
        match self.opt(k) {
            None => Ok(def),
            Some("on" | "true" | "yes" | "1") => Ok(true),
            Some("off" | "false" | "no" | "0") => Ok(false),
            Some(v) => err(
                self.head_line,
                format!("option {}={} must be on or off", k, v),
            ),
        }
    }

    pub fn opt_choice<'a>(&'a self, k: &str, choices: &[&'a str], def: &'a str) -> Res<&'a str> {
        match self.opt(k) {
            None => Ok(def),
            Some(v) if choices.contains(&v) => Ok(v),
            Some(v) => err(
                self.head_line,
                format!(
                    "invalid option {}={} (available: {})",
                    k,
                    v,
                    choices.join("|")
                ),
            ),
        }
    }

    pub fn opt_num(&self, k: &str) -> Res<Option<i64>> {
        match self.opt(k) {
            None => Ok(None),
            Some(v) => v.parse().map(Some).map_err(|_| {
                fail(
                    self.head_line,
                    format!("option {}={} must be an integer", k, v),
                )
            }),
        }
    }

    /// 逗号分隔的列表选项，如 `rows=A,B,C`。
    pub fn opt_list(&self, k: &str) -> Option<Vec<String>> {
        self.opt(k)
            .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
    }

    pub fn first_line(&self) -> usize {
        self.lines.first().map(|l| l.no).unwrap_or(self.head_line)
    }
}

fn strip_comment(s: &str) -> &str {
    let mut in_q = false;
    let mut esc = false;
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if in_q {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_q = false;
            }
        } else if c == b'"' {
            in_q = true;
        } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
            return &s[..i];
        }
    }
    s
}

pub fn parse_attrs(s: &str, no: usize) -> Res<Attrs> {
    let mut a = Attrs::default();
    for part in s.split(',') {
        let p = part.trim();
        match p {
            "" => {}
            "hl" => a.color = Some(Color::Hl),
            "red" => a.color = Some(Color::Red),
            "green" => a.color = Some(Color::Green),
            "blue" => a.color = Some(Color::Blue),
            "gray" | "grey" => a.color = Some(Color::Gray),
            "dashed" => a.dashed = true,
            "box" => a.boxed = true,
            _ => {
                if let Some(n) = p.strip_prefix('^') {
                    if n.is_empty() {
                        return err(no, "a pointer `^` needs a name, e.g. `^p`");
                    }
                    a.ptrs.push(n.to_string());
                } else {
                    return err(
                        no,
                        format!(
                            "unknown attribute `{}` (available: hl red green blue gray dashed box ^name)",
                            p
                        ),
                    );
                }
            }
        }
    }
    Ok(a)
}

/// 把一行切成记号。
pub fn lex(s: &str, no: usize) -> Res<Vec<Tok>> {
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    let read_attrs = |i: &mut usize| -> Res<Attrs> {
        // cs[*i] == '['
        let start = *i + 1;
        let mut j = start;
        while j < cs.len() && cs[j] != ']' {
            j += 1;
        }
        if j >= cs.len() {
            return err(no, "missing closing `]` in attributes");
        }
        *i = j + 1;
        let body: String = cs[start..j].iter().collect();
        parse_attrs(&body, no)
    };
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' {
            let mut t = String::new();
            i += 1;
            let mut closed = false;
            while i < cs.len() {
                let d = cs[i];
                if d == '\\' && i + 1 < cs.len() {
                    t.push(cs[i + 1]);
                    i += 2;
                    continue;
                }
                if d == '"' {
                    closed = true;
                    i += 1;
                    break;
                }
                t.push(d);
                i += 1;
            }
            if !closed {
                return err(no, "unclosed quote");
            }
            let mut attrs = Attrs::default();
            while i < cs.len() && cs[i] == '[' {
                attrs.merge(read_attrs(&mut i)?);
            }
            out.push(Tok::E(Elem {
                text: t,
                quoted: true,
                attrs,
            }));
            continue;
        }
        if c == '[' {
            let a = read_attrs(&mut i)?;
            out.push(Tok::Attrs(a));
            continue;
        }
        let start = i;
        while i < cs.len() && !cs[i].is_whitespace() && cs[i] != '[' {
            i += 1;
        }
        let w: String = cs[start..i].iter().collect();
        let mut attrs = Attrs::default();
        while i < cs.len() && cs[i] == '[' {
            attrs.merge(read_attrs(&mut i)?);
        }
        if let Some(rest) = w.strip_prefix('@') {
            if rest.is_empty() {
                return err(no, "`@` must be followed by a value");
            }
            out.push(Tok::At(rest.to_string()));
            continue;
        }
        let mut pieces = split_arrows(&w);
        if let Some(Tok::E(e)) = pieces.last_mut() {
            e.attrs = attrs;
        } else if attrs.color.is_some() || attrs.dashed || attrs.boxed || !attrs.ptrs.is_empty() {
            pieces.push(Tok::Attrs(attrs));
        }
        out.extend(pieces);
    }
    Ok(out)
}

fn split_arrows(w: &str) -> Vec<Tok> {
    match w {
        "-" | "--" => return vec![Tok::Dash],
        "->" => return vec![Tok::Arrow],
        "<->" => return vec![Tok::DArrow],
        _ => {}
    }
    let mut out = Vec::new();
    let mut rest = w;
    loop {
        let a = rest.find("<->");
        let b = rest.find("->");
        let (pos, len, tok) = match (a, b) {
            (Some(x), Some(y)) if x <= y => (x, 3, Tok::DArrow),
            (_, Some(y)) => (y, 2, Tok::Arrow),
            (Some(x), None) => (x, 3, Tok::DArrow),
            (None, None) => break,
        };
        if pos > 0 {
            out.push(word(&rest[..pos]));
        }
        out.push(tok);
        rest = &rest[pos + len..];
    }
    if !rest.is_empty() {
        out.push(word(rest));
    }
    out
}

fn word(s: &str) -> Tok {
    Tok::E(Elem {
        text: s.to_string(),
        quoted: false,
        attrs: Attrs::default(),
    })
}

pub fn parse(src: &str) -> Res<Doc> {
    let mut head: Option<(usize, Vec<String>)> = None;
    let mut lines = Vec::new();
    let mut raw: Vec<(usize, String)> = Vec::new();
    for (i, l) in src.lines().enumerate() {
        let no = i + 1;
        let t = strip_comment(l);
        if t.trim().is_empty() {
            continue;
        }
        if head.is_none() {
            head = Some((no, t.split_whitespace().map(|s| s.to_string()).collect()));
        } else {
            raw.push((no, t.trim_end().replace('\t', "    ")));
        }
    }
    let (head_line, words) = head.ok_or_else(|| {
        fail(
            0,
            "empty input; the first line should name a kind, e.g. `bintree`",
        )
    })?;
    let kind = words[0].clone();
    let mut opts = Vec::new();
    for w in &words[1..] {
        match w.split_once('=') {
            Some((k, v)) if !k.is_empty() => opts.push((k.to_string(), v.to_string())),
            _ => {
                return err(
                    head_line,
                    format!("header option `{}` should be written as key=value", w),
                )
            }
        }
    }
    let base = raw
        .iter()
        .map(|(_, s)| s.len() - s.trim_start().len())
        .min()
        .unwrap_or(0);
    for (no, s) in raw {
        let indent = s.len() - s.trim_start().len() - base;
        let toks = lex(&s, no)?;
        if !toks.is_empty() {
            lines.push(Line { no, indent, toks });
        }
    }
    Ok(Doc {
        kind,
        head_line,
        opts,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_elements() {
        let t = lex(r#"L: 头[gray] -> a_1 -> "a b"[^p,red] -> ^ @2"#, 1).unwrap();
        assert_eq!(t.len(), 9);
        match &t[1] {
            Tok::E(e) => {
                assert_eq!(e.text, "头");
                assert_eq!(e.attrs.color, Some(Color::Gray));
            }
            _ => panic!(),
        }
        match &t[5] {
            Tok::E(e) => {
                assert!(e.quoted);
                assert_eq!(e.text, "a b");
                assert_eq!(e.attrs.ptrs, vec!["p"]);
            }
            _ => panic!(),
        }
        assert!(matches!(t[8], Tok::At(ref s) if s == "2"));
    }

    #[test]
    fn splits_glued_arrows() {
        let t = lex("A->B<->C", 1).unwrap();
        assert_eq!(t.len(), 5);
        assert!(matches!(t[1], Tok::Arrow));
        assert!(matches!(t[3], Tok::DArrow));
    }

    #[test]
    fn negative_numbers_are_labels() {
        let t = lex("-1 3 -", 1).unwrap();
        assert!(matches!(&t[0], Tok::E(e) if e.text == "-1"));
        assert!(matches!(t[2], Tok::Dash));
    }

    #[test]
    fn comments_and_header() {
        let d = parse("// hi\nbintree thread=in // x\n\n  A\n    B \"//\"\n").unwrap();
        assert_eq!(d.kind, "bintree");
        assert_eq!(d.opt("thread"), Some("in"));
        assert_eq!(d.lines.len(), 2);
        assert_eq!(d.lines[0].indent, 0);
        assert_eq!(d.lines[1].indent, 2);
        assert_eq!(d.lines[1].no, 5);
        assert_eq!(d.lines[1].toks.len(), 2);
    }

    #[test]
    fn errors_have_line_numbers() {
        let e = parse("array\na[foo]").err().unwrap();
        assert!(e.contains("line 2"), "{}", e);
        assert!(e.contains("foo"));
    }
}
