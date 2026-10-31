//! dsviz：把数据结构 DSL 画成 SVG 的 Typst 插件。
//!
//! 两个入口：
//! - `labels(src)`：返回所有需要量宽度的文字，每行一个；
//! - `render(src, cfg)`：cfg 为 `键=值` 行和 `宽度\t文字` 行，返回 SVG。

mod block;
mod graph;
mod grid;
mod linear;
pub mod parse;
pub mod svg;
mod timing;
mod tree;

use svg::{Canvas, Config, Meas};

pub const KINDS: &[&str] = &[
    "tree",
    "forest",
    "bintree",
    "rbtree",
    "btree",
    "bplus",
    "list",
    "array",
    "stack",
    "ring",
    "matrix",
    "table",
    "graph",
    "digraph",
    "orthlist",
    "multilist",
    "fields",
    "pipeline",
    "gantt",
    "ladder",
    "wave",
    "block",
];

/// 解析并绘制，返回 SVG 文本。
pub fn run(src: &str, cfg: &Config, m: &Meas) -> Result<String, String> {
    let doc = parse::parse(src)?;
    let mut cv = Canvas::new(cfg, m);
    match doc.kind.as_str() {
        "tree" | "forest" => tree::tree(&doc, &mut cv)?,
        "bintree" => tree::bintree(&doc, &mut cv, false)?,
        "rbtree" => tree::bintree(&doc, &mut cv, true)?,
        "btree" => tree::btree(&doc, &mut cv, false)?,
        "bplus" => tree::btree(&doc, &mut cv, true)?,
        "list" => linear::list(&doc, &mut cv)?,
        "array" => linear::array(&doc, &mut cv)?,
        "stack" => linear::stack(&doc, &mut cv)?,
        "ring" => linear::ring(&doc, &mut cv)?,
        "matrix" => grid::matrix(&doc, &mut cv)?,
        "table" => grid::table(&doc, &mut cv)?,
        "graph" => graph::graph(&doc, &mut cv, false)?,
        "digraph" => graph::graph(&doc, &mut cv, true)?,
        "orthlist" => graph::orthlist(&doc, &mut cv, false)?,
        "multilist" => graph::orthlist(&doc, &mut cv, true)?,
        "fields" => timing::fields(&doc, &mut cv)?,
        "pipeline" => timing::pipeline(&doc, &mut cv)?,
        "gantt" => timing::gantt(&doc, &mut cv)?,
        "ladder" => timing::ladder(&doc, &mut cv)?,
        "wave" => timing::wave(&doc, &mut cv)?,
        "block" => block::block(&doc, &mut cv)?,
        k => {
            return parse::err(
                doc.head_line,
                format!("unknown kind `{}` (available: {})", k, KINDS.join(" ")),
            )
        }
    }
    Ok(cv.finish())
}

/// 第一遍：收集要量宽度的文字。
pub fn labels_of(src: &str) -> Result<String, String> {
    let cfg = Config::default();
    let m = Meas::recording(cfg.size);
    run(src, &cfg, &m)?;
    Ok(m.recorded().join("\n"))
}

/// 第二遍：按实测宽度出图。
pub fn render_with(src: &str, cfg: &str) -> Result<String, String> {
    let cfg = Config::parse(cfg)?;
    let m = Meas::from_cfg(&cfg);
    run(src, &cfg, &m)
}

#[cfg(target_arch = "wasm32")]
mod plugin {
    use wasm_minimal_protocol::*;

    initiate_protocol!();

    fn utf8(b: &[u8]) -> Result<&str, String> {
        std::str::from_utf8(b).map_err(|_| "dsviz: input is not valid UTF-8".to_string())
    }

    #[wasm_func]
    pub fn labels(src: &[u8]) -> Result<Vec<u8>, String> {
        super::labels_of(utf8(src)?).map(String::into_bytes)
    }

    #[wasm_func]
    pub fn render(src: &[u8], cfg: &[u8]) -> Result<Vec<u8>, String> {
        super::render_with(utf8(src)?, utf8(cfg)?).map(String::into_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_kind() {
        let e = labels_of("foo\nA").unwrap_err();
        assert!(e.contains("line 1") && e.contains("foo"), "{}", e);
    }

    #[test]
    fn every_kind_renders() {
        let cases = [
            "tree\nA\n  B\n  C\n    D\nE",
            "bintree thread=in\nA\n  B\n    _\n    D\n  C",
            "bintree bits=on ext=on\nA B C",
            "rbtree nil=on\n10\n  5[red]\n  20",
            "btree ext=on\n30\n  10 20\n  40 50",
            "bintree layout=tidy ext=on\n<5: 5\n  <2: 2[box]\n  >2: 8",
            "tree\n\"a b\"[box]\n  x: c\n  d",
            "bplus\n20 40\n  5 20\n  30 40",
            "list\nL: 头[gray] -> a_1 -> a_2[^p] -> ^\nx <-> y <-> z @1",
            "array index=0\n第1趟: 3 1[hl] _ 2[^low]",
            "stack index=0 size=4\na b[^top]",
            "ring size=6\n_ a[^front] b _[^rear]",
            "matrix labels=A,B\n0 1\n1 .",
            "table head=row\nj 1 2\nnext 0 1",
            "graph\nA - B 5\nB - C - A",
            "digraph dir=lr\nA -> B\nB -> A\nA -> C 3 [red]\nC -> C",
            "graph\nA @0,0\nB @1,0\nA - B",
            "orthlist\nA -> B\nA -> C\nC -> A\nB -> C",
            "multilist\nA - B\nA - C\nB - C\nB - D",
            "fields ticks=on\n虚拟地址: 页目录号:10 | 页表索引:10[hl] | 页内偏移:12\nOP:6|Rs:5|Rd:5|imm:16",
            "fields\n\"单 精度\": S:1 | \"阶 码\":8[blue] | _:23",
            "pipeline cycles=8\nI1: IF ID EX M WB\nI2: _ IF ID stall EX M WB\nI3: _ _ IF * ID EX\nI1.M -> I2.EX\nI1.M -> I3.ID [dashed]",
            "gantt\nP1: 0-3 7-9\nP2: 3-7[hl]",
            "gantt axis=off\nCPU: 0-3:P1 3-7:\"就绪 队列\"[red]",
            "gantt\n0-2.5:P_1 2.5-4:P_2",
            "ladder actors=A,B slope=1\nA -> B \"SYN, seq=x\"\nnote A \"SYN-SENT\"\nB -> A \"SYN+ACK\"\nA -x B \"数据\"\ngap\nA => B \"重传\" [dashed]\nnote B @2 \"S\"",
            "ladder actors=A,B,C\nA -> C \"x\" @1\nC -> A \"y\" @1",
            "wave grid=on bits=1 0 1\nCLK: c c c\n曼彻斯特: 10 01 10\n地址: =A1 . x\n读: 1 0 z\n数据: =\"a b\"[hl] \"c d\" clk",
            "block\nR1 router @0,0\nR2 router @3,0\nS1 switch @0,2\nH1 host @-1,3\nN cloud \"Internet\" @5,0 w=1.5\nR1 - R2 \"E0: 1.1.1.1\" \"E1: 1.1.1.2\"\nR1 - S1\nS1 - H1\nR2 - N\nB bus \"内部总线\" @0,5 w=8\nPC reg @1,4\nPC.s - B",
            "block\nA @0,0\nM mem \"内存\" @3,0 cells=4 [hl]\nU alu @0,2\nX mux @3,2\nS server @5,0\nH hub @5,2\nA.e2 -> M.w3 \"x\"\nA.e1 -> M.w1\nA <-> U [dashed]\nU -> X [straight]\ntext @1,1 \"文字\"",
        ];
        for c in cases {
            let l = labels_of(c).unwrap_or_else(|e| panic!("{}: {}", c, e));
            let cfg: String = l.lines().map(|s| format!("5\t{}\n", s)).collect();
            let svg = render_with(c, &format!("size=10\n{}", cfg)).unwrap();
            assert!(svg.contains("\u{1}<svg"), "{}", c);
            assert!(
                !svg.contains("NaN") && !svg.contains("inf"),
                "{}\n{}",
                c,
                svg
            );
        }
    }

    #[test]
    fn new_kind_errors() {
        for (src, want) in [
            ("fields\nOP | B:2", "line 2"),
            ("fields\nA:x", "invalid width"),
            ("pipeline cycles=2\nI1: IF ID EX", "cycles=2"),
            ("pipeline\nI1: IF\nI1.EX -> I1.IF", "no stage"),
            ("pipeline\nI1: IF\nI9.IF -> I1.IF", "unknown instruction"),
            ("gantt\nP1: 5-3", "end after"),
            ("ladder\nA -> B", "actors"),
            ("ladder actors=A\nnote A \"x\"", "2 to 5"),
            ("ladder actors=A,B\nA -> C \"x\"", "unknown actor"),
            ("wave\nA: 2", "unknown cycle"),
            ("wave\nA: . 1", "continues"),
            ("block\nA box", "coordinate"),
            ("block\nA @0,0\nA - B", "unknown node"),
            ("block\nA @0,0\nB @1,0\nA.q - B", "bad port"),
        ] {
            let e = labels_of(src).unwrap_err();
            assert!(e.contains(want), "{}: {}", src, e);
        }
    }

    #[test]
    fn other_kinds_still_reject_spaced_header_words() {
        assert!(labels_of("array bits=1 0\na").is_err());
    }

    #[test]
    fn labels_collects_subscripts() {
        let l = labels_of("array\na_1 b").unwrap();
        let v: Vec<&str> = l.lines().collect();
        assert!(v.contains(&"a") && v.contains(&"1") && v.contains(&"b"));
    }
}
