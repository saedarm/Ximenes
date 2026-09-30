//! Output: terminal text, SVG, and a dependency-free PDF (standard Helvetica,
//! so nothing is embedded and the file stays a few KB).

use crate::clue::Clue;
use crate::grid::{Fill, N};

pub struct Puzzle {
    pub title: String,
    pub code: String,
    pub fill: Fill,
    pub clues: Vec<Clue>,
}

impl Puzzle {
    fn numbers(&self) -> [[usize; N]; N] {
        let mut nums = [[0; N]; N];
        for e in &self.fill.entries {
            let (r, c) = e.slot.cells[0];
            nums[r][c] = e.slot.number;
        }
        nums
    }

    fn sorted(&self, across: bool) -> Vec<&Clue> {
        let mut v: Vec<&Clue> = self.clues.iter().filter(|c| c.across == across).collect();
        v.sort_by_key(|c| c.number);
        v
    }
}

// ---------------------------------------------------------------- text

pub fn text(p: &Puzzle, reveal: bool) -> String {
    let mut s = format!("{}\nshare code: {}\n\n", p.title, p.code);
    s += &text_grid(p, false);
    for across in [true, false] {
        s += if across { "\nACROSS\n" } else { "\nDOWN\n" };
        for c in p.sorted(across) {
            s += &format!("{:>3}  {} {}\n", c.number, c.text, c.enumeration);
        }
    }
    if reveal {
        s += "\nSOLUTION\n\n";
        s += &text_grid(p, true);
        s += "\n";
        for across in [true, false] {
            for c in p.sorted(across) {
                let star = if c.theme { " *theme" } else { "" };
                s += &format!("{:>2}{}  {}{}\n     {}\n", c.number, if across { "a" } else { "d" }, c.answer, star, c.explain);
            }
        }
    }
    s
}

fn text_grid(p: &Puzzle, letters: bool) -> String {
    let nums = p.numbers();
    let rule = format!("+{}\n", "---+".repeat(N));
    let mut s = rule.clone();
    for r in 0..N {
        let mut top = String::from("|");
        let mut mid = String::from("|");
        for c in 0..N {
            if p.fill.blocks[r][c] {
                top += "###|";
                mid += "###|";
            } else {
                top += &if nums[r][c] > 0 { format!("{:<3}|", nums[r][c]) } else { "   |".into() };
                mid += &if letters { format!(" {} |", p.fill.letters[r][c] as char) } else { "   |".into() };
            }
        }
        s += &format!("{top}\n{mid}\n{rule}");
    }
    s
}

// ---------------------------------------------------------------- metrics

/// Helvetica advance widths (1/1000 em) for ASCII 32..=126.
const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

fn width(s: &str, size: f64, bold: bool) -> f64 {
    let em: f64 = s
        .chars()
        .map(|c| match c as u32 {
            32..=126 => HELV[c as usize - 32] as f64,
            0x2018 | 0x2019 => 222.0,
            _ => 556.0,
        })
        .sum();
    em * size / 1000.0 * if bold { 1.07 } else { 1.0 }
}

fn wrap(s: &str, size: f64, max: f64) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let trial = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if width(&trial, size, false) > max && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        } else {
            cur = trial;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

// ---------------------------------------------------------------- shared layout

/// A drawing op in a y-down coordinate space; both backends replay these.
enum Op {
    Rect { x: f64, y: f64, w: f64, h: f64, fill: bool, stroke: f64 },
    Text { x: f64, y: f64, size: f64, bold: bool, italic: bool, s: String },
}

struct Page {
    ops: Vec<Op>,
}

const PAGE_W: f64 = 612.0;
const PAGE_H: f64 = 792.0;
const MARGIN: f64 = 54.0;

fn draw_grid(ops: &mut Vec<Op>, p: &Puzzle, x0: f64, y0: f64, cell: f64, letters: bool) {
    let nums = p.numbers();
    for r in 0..N {
        for c in 0..N {
            let (x, y) = (x0 + c as f64 * cell, y0 + r as f64 * cell);
            let block = p.fill.blocks[r][c];
            ops.push(Op::Rect { x, y, w: cell, h: cell, fill: block, stroke: 0.6 });
            if block {
                continue;
            }
            if nums[r][c] > 0 {
                ops.push(Op::Text { x: x + 2.0, y: y + cell * 0.28, size: cell * 0.27, bold: false, italic: false, s: nums[r][c].to_string() });
            }
            if letters {
                let ch = (p.fill.letters[r][c] as char).to_string();
                let size = cell * 0.55;
                ops.push(Op::Text { x: x + (cell - width(&ch, size, true)) / 2.0, y: y + cell * 0.82, size, bold: true, italic: false, s: ch });
            }
        }
    }
    let side = cell * N as f64;
    ops.push(Op::Rect { x: x0, y: y0, w: side, h: side, fill: false, stroke: 1.6 });
}

/// Flows text blocks into a column, breaking pages as needed.
struct Flow {
    pages: Vec<Page>,
    x: f64,
    w: f64,
    y: f64,
    footer: String,
    /// Below this y the column may use the full page width (it has cleared the grid).
    widen_below: f64,
}

impl Flow {
    fn page(&mut self) -> &mut Vec<Op> {
        &mut self.pages.last_mut().unwrap().ops
    }

    fn need(&mut self, h: f64) {
        if self.y + h > PAGE_H - MARGIN {
            let footer = self.footer.clone();
            self.pages.push(new_page(&footer));
            self.x = MARGIN;
            self.w = PAGE_W - 2.0 * MARGIN;
            self.y = MARGIN;
            self.widen_below = 0.0;
        }
    }

    fn heading(&mut self, s: &str) {
        self.need(30.0);
        self.y += 18.0;
        let (x, y) = (self.x, self.y);
        self.page().push(Op::Text { x, y, size: 12.0, bold: true, italic: false, s: s.into() });
        self.y += 8.0;
    }

    /// `label` is set in bold in a gutter; `body` wraps beside it.
    fn item(&mut self, label: &str, body: &str, size: f64, gutter: f64) {
        let lead = size * 1.3;
        if self.y > self.widen_below {
            self.x = MARGIN;
            self.w = PAGE_W - 2.0 * MARGIN;
        }
        let lines = wrap(body, size, self.w - gutter);
        self.need(lead * lines.len() as f64 + 4.0);
        let x = self.x;
        for (i, line) in lines.iter().enumerate() {
            self.y += lead;
            let y = self.y;
            if i == 0 && !label.is_empty() {
                self.page().push(Op::Text { x, y, size, bold: true, italic: false, s: label.into() });
            }
            self.page().push(Op::Text { x: x + gutter, y, size, bold: false, italic: false, s: line.clone() });
        }
        self.y += 4.0;
    }
}

fn new_page(footer: &str) -> Page {
    Page {
        ops: vec![Op::Text { x: MARGIN, y: PAGE_H - 30.0, size: 8.0, bold: false, italic: true, s: footer.into() }],
    }
}

fn layout(p: &Puzzle, answers: bool) -> Vec<Page> {
    let footer = format!("Ximenes  \u{2022}  regenerate with: ximenes open {}", p.code);
    let mut first = new_page(&footer);
    first.ops.push(Op::Text { x: MARGIN, y: MARGIN + 6.0, size: 20.0, bold: true, italic: false, s: p.title.clone() });
    first.ops.push(Op::Text { x: MARGIN, y: MARGIN + 22.0, size: 9.0, bold: false, italic: true, s: "A 7x7 cryptic crossword".into() });
    let cell = 30.0;
    let top = MARGIN + 40.0;
    draw_grid(&mut first.ops, p, MARGIN, top, cell, false);

    let col_x = MARGIN + cell * N as f64 + 28.0;
    let mut flow = Flow { pages: vec![first], x: col_x, w: PAGE_W - MARGIN - col_x, y: top - 18.0, footer: footer.clone(), widen_below: f64::INFINITY };
    for across in [true, false] {
        flow.heading(if across { "Across" } else { "Down" });
        for c in p.sorted(across) {
            flow.item(&c.number.to_string(), &format!("{} {}", c.text, c.enumeration), 10.5, 20.0);
        }
    }

    if answers {
        let mut sol = new_page(&footer);
        sol.ops.push(Op::Text { x: MARGIN, y: MARGIN + 6.0, size: 16.0, bold: true, italic: false, s: format!("Solution \u{2014} {}", p.title) });
        let cell = 20.0;
        draw_grid(&mut sol.ops, p, MARGIN, MARGIN + 24.0, cell, true);
        flow.pages.push(sol);
        flow.x = MARGIN + cell * N as f64 + 24.0;
        flow.w = PAGE_W - MARGIN - flow.x;
        flow.y = MARGIN + 6.0;
        flow.widen_below = f64::INFINITY;
        for across in [true, false] {
            flow.heading(if across { "Across" } else { "Down" });
            for c in p.sorted(across) {
                let theme = if c.theme { "  [theme]" } else { "" };
                flow.item(&format!("{}{}", c.number, if across { "a" } else { "d" }), &format!("{}{theme} \u{2014} {}", c.answer, c.explain), 9.0, 22.0);
            }
        }
    }
    flow.pages
}

// ---------------------------------------------------------------- SVG

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn svg(p: &Puzzle, answers: bool) -> String {
    let pages = layout(p, answers);
    let total_h = PAGE_H * pages.len() as f64;
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{PAGE_W}\" height=\"{total_h}\" viewBox=\"0 0 {PAGE_W} {total_h}\" font-family=\"Helvetica, Arial, sans-serif\">\n<rect width=\"100%\" height=\"100%\" fill=\"#fff\"/>\n"
    );
    for (i, page) in pages.iter().enumerate() {
        let dy = i as f64 * PAGE_H;
        if i > 0 {
            s += &format!("<line x1=\"0\" y1=\"{dy}\" x2=\"{PAGE_W}\" y2=\"{dy}\" stroke=\"#bbb\" stroke-dasharray=\"4 4\"/>\n");
        }
        for op in &page.ops {
            match op {
                Op::Rect { x, y, w, h, fill, stroke } => {
                    s += &format!(
                        "<rect x=\"{x:.1}\" y=\"{:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"{}\" stroke=\"#000\" stroke-width=\"{stroke}\"/>\n",
                        y + dy,
                        if *fill { "#000" } else { "none" }
                    )
                }
                Op::Text { x, y, size, bold, italic, s: t } => {
                    s += &format!(
                        "<text x=\"{x:.1}\" y=\"{:.1}\" font-size=\"{size:.1}\"{}{}>{}</text>\n",
                        y + dy,
                        if *bold { " font-weight=\"bold\"" } else { "" },
                        if *italic { " font-style=\"italic\" fill=\"#555\"" } else { "" },
                        xml(t)
                    )
                }
            }
        }
    }
    s + "</svg>\n"
}

// ---------------------------------------------------------------- PDF

fn pdf_str(s: &str) -> Vec<u8> {
    let mut out = vec![b'('];
    for ch in s.chars() {
        let b: u8 = match ch {
            '(' | ')' | '\\' => {
                out.push(b'\\');
                ch as u8
            }
            '\u{2018}' => 0x91,
            '\u{2019}' => 0x92,
            '\u{201C}' => 0x93,
            '\u{201D}' => 0x94,
            '\u{2022}' => 0x95,
            '\u{2013}' => 0x96,
            '\u{2014}' => 0x97,
            c if (c as u32) < 256 => c as u8,
            _ => b'?',
        };
        out.push(b);
    }
    out.push(b')');
    out
}

pub fn pdf(p: &Puzzle) -> Vec<u8> {
    let pages = layout(p, true);
    let mut objs: Vec<Vec<u8>> = Vec::new();
    // 1 catalog, 2 pages, 3-5 fonts, then (page, content) pairs
    let n = pages.len();
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 6 + 2 * i)).collect();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes());
    for font in ["Helvetica", "Helvetica-Bold", "Helvetica-Oblique"] {
        objs.push(format!("<< /Type /Font /Subtype /Type1 /BaseFont /{font} /Encoding /WinAnsiEncoding >>").into_bytes());
    }
    for (i, page) in pages.iter().enumerate() {
        let mut c: Vec<u8> = Vec::new();
        for op in &page.ops {
            match op {
                Op::Rect { x, y, w, h, fill, stroke } => {
                    let py = PAGE_H - y - h;
                    if *fill {
                        c.extend(format!("0 g {x:.2} {py:.2} {w:.2} {h:.2} re f\n").as_bytes());
                    }
                    c.extend(format!("{stroke} w 0 G {x:.2} {py:.2} {w:.2} {h:.2} re S\n").as_bytes());
                }
                Op::Text { x, y, size, bold, italic, s } => {
                    let f = if *bold { "F2" } else if *italic { "F3" } else { "F1" };
                    let gray = if *italic { "0.35 g" } else { "0 g" };
                    c.extend(format!("{gray} BT /{f} {size:.2} Tf {x:.2} {:.2} Td ", PAGE_H - y).as_bytes());
                    c.extend(pdf_str(s));
                    c.extend(b" Tj ET\n");
                }
            }
        }
        objs.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents {} 0 R >>",
                7 + 2 * i
            )
            .into_bytes(),
        );
        let mut stream = format!("<< /Length {} >>\nstream\n", c.len()).into_bytes();
        stream.extend(c);
        stream.extend(b"\nendstream");
        objs.push(stream);
    }

    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend(o);
        out.extend(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for off in offsets {
        out.extend(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}
