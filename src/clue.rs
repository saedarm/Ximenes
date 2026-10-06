//! Writes one cryptic clue per entry and records how it works.
//!
//! Every device builds its wordplay from the baked-in word list, so the
//! explanation is a literal account of the construction, not a guess.

use crate::dict::{Dict, Part, sorted_key};
use crate::grid::Entry;
use crate::rng::Rng;
use crate::theme::ThemeEntry;
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Device {
    Anagram,
    Hidden,
    Charade,
    Container,
    Reversal,
    Deletion,
    DoubleDef,
    AndLit,
    DefinitionOnly,
}

impl Device {
    pub fn name(self) -> &'static str {
        match self {
            Device::Anagram => "Anagram",
            Device::Hidden => "Hidden word",
            Device::Charade => "Charade",
            Device::Container => "Container",
            Device::Reversal => "Reversal",
            Device::Deletion => "Deletion",
            Device::DoubleDef => "Double definition",
            Device::AndLit => "&lit",
            Device::DefinitionOnly => "Definition only",
        }
    }
}

pub struct Clue {
    pub number: usize,
    pub across: bool,
    pub text: String,
    pub enumeration: String,
    pub answer: String,
    pub device: Device,
    pub explain: String,
    pub theme: bool,
}

/// Wordplay without its definition yet.
struct Play {
    device: Device,
    text: String,
    explain: String,
    score: f64,
}

/// A finished candidate clue.
struct Cand {
    device: Device,
    text: String,
    explain: String,
    score: f64,
}

const ANAGRAM_POST: &[&str] = &[
    "wildly",
    "badly",
    "broken",
    "mixed",
    "confused",
    "off",
    "out",
    "ruined",
    "shattered",
    "scrambled",
    "rearranged",
    "twisted",
    "at sea",
    "all over",
    "in a mess",
    "in trouble",
    "reformed",
    "abroad",
    "strangely",
    "cooked",
    "loose",
    "stirred",
    "dancing",
    "madly",
    "in pieces",
    "shaken",
    "tossed",
    "out of order",
    "awkwardly",
    "adrift",
];
const ANAGRAM_PRE: &[&str] = &[
    "crazy",
    "broken",
    "wild",
    "strange",
    "new",
    "twisted",
    "reworked",
    "battered",
    "jumbled",
    "mangled",
    "novel",
    "unusual",
    "awful",
    "odd-looking",
    "restless",
];
const ANDLIT_IND: &[&str] = &[
    "wildly",
    "madly",
    "badly",
    "strangely",
    "crazily",
    "awfully",
];
const HIDDEN_DEF_FIRST: &[&str] = &[
    "in",
    "within",
    "found in",
    "held by",
    "hidden in",
    "inside",
    "concealed by",
    "housed in",
];
const HIDDEN_LEAD: &[&str] = &["Some", "Part of", "A bit of", "Partly", "Some of"];
const HIDDEN_VERB: &[&str] = &[
    "hides", "holds", "contains", "conceals", "keeps", "harbours",
];
const CONTAIN_IN: &[&str] = &[
    "in",
    "inside",
    "held by",
    "within",
    "kept by",
    "swallowed by",
    "grabbed by",
];
const CONTAIN_OUT: &[&str] = &[
    "around",
    "holding",
    "about",
    "grabbing",
    "outside",
    "keeping",
    "clutching",
    "boxing",
];
const REVERSE_ACROSS: &[&str] = &[
    "back",
    "returned",
    "reversed",
    "turned back",
    "retreating",
    "going west",
    "in retreat",
];
const REVERSE_DOWN: &[&str] = &["up", "rising", "going up", "raised", "lifted", "overturned"];
const DELETE_IND: &[&str] = &[
    "without",
    "losing",
    "dropping",
    "lacking",
    "minus",
    "shedding",
    "leaving out",
    "having lost",
];
const BEHEAD: &[&str] = &[
    "{w} beheaded",
    "headless {w}",
    "{w}, topless",
    "{w} losing its head",
];
const CURTAIL: &[&str] = &[
    "{w} cut short",
    "endless {w}",
    "{w} almost",
    "{w}, not finished",
    "{w} docked",
];
const LINK_DEF_LAST: &[&str] = &[
    "",
    "",
    "for",
    "gives",
    "is",
    "to make",
    "making",
    "producing",
];
const LINK_DEF_FIRST: &[&str] = &["", "", "from", "is", "gets", "needs"];

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn q(s: &str) -> String {
    format!("\u{2018}{s}\u{2019}")
}

fn join3(a: &str, link: &str, b: &str) -> String {
    if link.is_empty() {
        format!("{a} {b}")
    } else {
        format!("{a} {link} {b}")
    }
}

/// Definition text for a clue part, picked from its list.
fn part_text(p: &Part, rng: &mut Rng) -> &'static str {
    // favour the first (most standard) reading
    let w: Vec<f64> = (0..p.defs.len()).map(|i| 1.0 / (1 + i) as f64).collect();
    p.defs[rng.weighted(&w)]
}

fn part_explain(p: &Part, text: &str) -> String {
    if text.eq_ignore_ascii_case(&p.letters) {
        format!("{} (as is)", p.letters)
    } else {
        format!("{} ({})", p.letters, q(text))
    }
}

pub struct Setter<'a> {
    pub dict: &'a Dict,
}

impl Setter<'_> {
    /// (definition, WordNet lexname, is a true synonym). A theme answer's own
    /// definition comes first and has no lexname.
    fn defs_for(
        &self,
        entry: &Entry,
        themes: &[ThemeEntry],
    ) -> Vec<(String, Option<&'static str>, bool)> {
        let mut out = Vec::new();
        if let Some(ti) = entry.theme {
            out.push((themes[ti].def.clone(), None, true));
        }
        if let Some(w) = self.dict.get(&entry.answer) {
            for s in w.senses.iter().take(3) {
                for (d, direct) in
                    s.defs
                        .iter()
                        .zip(&s.direct)
                        .take(if entry.theme.is_some() { 1 } else { 2 })
                {
                    out.push((d.to_string(), Some(s.lex), *direct));
                }
            }
        }
        out
    }

    fn anagram_fodder(&self, ans: &str) -> Vec<(Vec<&'static str>, f64)> {
        let d = self.dict;
        let bad = |w: &str| w == ans || w.contains(ans) || ans.contains(w) && w.len() > 3;
        let key = sorted_key(ans);
        let mut out = Vec::new();
        for &i in d.anagrams_of(&key) {
            let w = &d.words[i];
            if !bad(w.text) && w.text[..3] != ans[..3] {
                out.push((vec![w.text], 4.5 + w.freq as f64 / 3.0));
            }
        }
        // two-word fodder, or "a"/"I" plus a word
        let n = key.len();
        let mut seen = BTreeSet::new();
        for mask in 1u32..(1 << n) - 1 {
            let pick: Vec<u8> = (0..n)
                .filter(|b| mask >> b & 1 == 1)
                .map(|b| key[b])
                .collect();
            let rest: Vec<u8> = (0..n)
                .filter(|b| mask >> b & 1 == 0)
                .map(|b| key[b])
                .collect();
            if pick > rest || !seen.insert(pick.clone()) {
                continue;
            }
            let lists = |k: &[u8]| -> Vec<(&'static str, f32)> {
                if k == b"A" {
                    return vec![("A", 7.0)];
                }
                if k == b"I" {
                    return vec![("I", 7.0)];
                }
                if k.len() < 2 {
                    return vec![];
                }
                let mut v: Vec<(&str, f32)> = d
                    .anagrams_of(k)
                    .iter()
                    .map(|&i| (d.words[i].text, d.words[i].freq))
                    .collect();
                v.sort_by(|a, b| b.1.total_cmp(&a.1));
                v.truncate(4);
                v
            };
            for (a, fa) in lists(&pick) {
                for (b, fb) in lists(&rest) {
                    if bad(a) || bad(b) {
                        continue;
                    }
                    let f = fa.min(fb) as f64;
                    // fodder that spells the answer in order is too easy
                    let lazy = format!("{a}{b}") == ans || format!("{b}{a}") == ans;
                    if f >= 4.0 && !lazy {
                        out.push((vec![a, b], 2.5 + f / 2.5));
                    }
                }
            }
        }
        out.sort_by(|a, b| b.1.total_cmp(&a.1));
        out.truncate(8);
        out
    }

    fn anagrams(&self, ans: &str, rng: &mut Rng) -> Vec<Play> {
        self.anagram_fodder(ans)
            .into_iter()
            .map(|(words, score)| {
                let fodder = words
                    .iter()
                    .map(|w| {
                        if *w == "I" {
                            "I".into()
                        } else {
                            w.to_lowercase()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let (text, ind) = if rng.chance(0.6) {
                    let ind = rng.pick(ANAGRAM_POST);
                    (format!("{fodder} {ind}"), ind)
                } else {
                    let ind = rng.pick(ANAGRAM_PRE);
                    (format!("{ind} {fodder}"), ind)
                };
                let explain = format!(
                    "anagram of {} (signalled by {}) gives {ans}",
                    words.join(" + "),
                    q(ind)
                );
                Play {
                    device: Device::Anagram,
                    text,
                    explain,
                    score,
                }
            })
            .collect()
    }

    fn and_lits(&self, ans: &str, rng: &mut Rng) -> Vec<Cand> {
        let d = self.dict;
        let Some(aw) = d.get(ans) else { return vec![] };
        let lower = ans.to_lowercase();
        let mut out = Vec::new();
        for &i in d.anagrams_of(&sorted_key(ans)) {
            let f = &d.words[i];
            if f.text == ans {
                continue;
            }
            if f.text[..3] == ans[..3] {
                continue; // spelling variants (FIBER/FIBRE) are not wordplay
            }
            let fl = f.text.to_lowercase();
            let direct = |w: &crate::dict::Word| -> Vec<&'static str> {
                w.senses
                    .iter()
                    .flat_map(|s| s.defs.iter().zip(&s.direct).filter(|p| *p.1).map(|p| *p.0))
                    .collect()
            };
            let (da, df) = (direct(aw), direct(f));
            let related = da.contains(&fl.as_str())
                || df.contains(&lower.as_str())
                || da.iter().any(|d| df.contains(d));
            if related {
                let ind = rng.pick(ANDLIT_IND);
                out.push(Cand {
                    device: Device::AndLit,
                    text: format!("{} {ind}!", cap(&fl)),
                    explain: format!(
                        "&lit: the whole clue is the definition ({} said {ind} is to be {lower}) and also the wordplay: anagram of {} (signalled by {}) gives {ans}",
                        fl, f.text, q(ind)
                    ),
                    score: 9.0,
                });
            }
        }
        out
    }

    fn hiddens(&self, ans: &str) -> Vec<Play> {
        let d = self.dict;
        let n = ans.len();
        let top = |ids: &[usize]| -> Vec<&'static str> {
            let mut v: Vec<(&str, f32)> = ids
                .iter()
                .map(|&i| (d.words[i].text, d.words[i].freq))
                .filter(|(w, _)| !w.contains(ans))
                .collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            v.truncate(12);
            v.into_iter().map(|x| x.0).collect()
        };
        let starting = |p: &str| -> Vec<&'static str> {
            let mut v = top(d.with_prefix(p));
            if d.is_surface_word(p)
                && p != ans
                && let Some(w) = d.get(p)
            {
                v.insert(0, w.text);
            }
            v
        };
        let freq = |w: &str| d.get(w).map_or(0.0, |x| x.freq as f64);
        let mut out: Vec<(Vec<&str>, usize, f64)> = Vec::new();
        for k in 1..n {
            let (head, rest) = (&ans[..k], &ans[k..]);
            for w1 in top(d.with_suffix(head))
                .into_iter()
                .filter(|w| w.len() >= 3)
                .take(6)
            {
                for w2 in starting(rest).into_iter().take(6) {
                    if w1.len() + w2.len() < 7 {
                        continue; // too thin to hide anything
                    }
                    let f = freq(w1).min(freq(w2));
                    out.push((vec![w1, w2], w1.len() - k, 2.5 + f / 4.0));
                }
                for m in 2..rest.len() {
                    let mid = &rest[..m];
                    if !d.is_surface_word(mid) {
                        continue;
                    }
                    for w3 in starting(&rest[m..]).into_iter().take(4) {
                        let f = freq(w1).min(freq(mid)).min(freq(w3));
                        out.push((
                            vec![w1, d.get(mid).unwrap().text, w3],
                            w1.len() - k,
                            1.8 + f / 4.0,
                        ));
                    }
                }
            }
        }
        out.sort_by(|a, b| b.2.total_cmp(&a.2));
        out.truncate(6);
        out.into_iter()
            .map(|(words, offset, score)| {
                let phrase = words.join(" ").to_lowercase();
                // show the answer picked out in capitals
                let mut shown = String::new();
                let mut letter = 0;
                for ch in phrase.chars() {
                    if ch == ' ' {
                        shown.push(ch);
                        continue;
                    }
                    let inside = letter >= offset && letter < offset + n;
                    shown.push(if inside { ch.to_ascii_uppercase() } else { ch });
                    letter += 1;
                }
                Play {
                    device: Device::Hidden,
                    text: phrase,
                    explain: format!("{ans} is hidden in {}", q(&shown)),
                    score,
                }
            })
            .collect()
    }

    /// All ways to split `s` into 2 or 3 clue-able parts.
    fn splits(&self, s: &str, max_parts: usize) -> Vec<Vec<Part>> {
        let mut out = Vec::new();
        for i in 1..s.len() {
            let Some(a) = self.dict.part(&s[..i]) else {
                continue;
            };
            if let Some(b) = self.dict.part(&s[i..]) {
                out.push(vec![a.clone(), b]);
            }
            if max_parts >= 3 {
                for j in i + 1..s.len() {
                    if let (Some(b), Some(c)) = (self.dict.part(&s[i..j]), self.dict.part(&s[j..]))
                    {
                        out.push(vec![a.clone(), b, c]);
                    }
                }
            }
        }
        out
    }

    fn parts_score(parts: &[Part]) -> f64 {
        let singles = parts.iter().filter(|p| p.letters.len() == 1).count();
        let words = parts.iter().filter(|p| !p.abbrev).count();
        2.5 + words as f64 * 1.3 - singles as f64 * 0.7 - (parts.len() as f64 - 2.0) * 0.8
    }

    fn charades(&self, ans: &str, down: bool, rng: &mut Rng) -> Vec<Play> {
        let mut out = Vec::new();
        for parts in self.splits(ans, 3) {
            if parts.iter().filter(|p| p.letters.len() == 1).count() > 1 {
                continue;
            }
            let t: Vec<&str> = parts.iter().map(|p| part_text(p, rng)).collect();
            let text = if parts.len() == 2 {
                let mut forms = vec![
                    "{a} {b}",
                    "{a} and {b}",
                    "{a} with {b}",
                    "{a} before {b}",
                    "{a} then {b}",
                    "{b} after {a}",
                    "{a} by {b}",
                ];
                if down {
                    forms.extend(["{a} on {b}", "{a} over {b}", "{a} above {b}"]);
                }
                rng.pick(&forms).replace("{a}", t[0]).replace("{b}", t[1])
            } else {
                let forms = [
                    "{a}, {b} and {c}",
                    "{a} {b} {c}",
                    "{a} with {b} and {c}",
                    "{a} and {b} then {c}",
                ];
                rng.pick(&forms)
                    .replace("{a}", t[0])
                    .replace("{b}", t[1])
                    .replace("{c}", t[2])
            };
            let explain = parts
                .iter()
                .zip(&t)
                .map(|(p, t)| part_explain(p, t))
                .collect::<Vec<_>>()
                .join(" + ");
            out.push(Play {
                device: Device::Charade,
                text,
                explain: format!("{explain} = {ans}"),
                score: Self::parts_score(&parts),
            });
        }
        out
    }

    fn containers(&self, ans: &str, rng: &mut Rng) -> Vec<Play> {
        let mut out = Vec::new();
        let n = ans.len();
        for i in 1..n - 1 {
            for j in i + 1..n {
                let inner = &ans[i..j];
                let outer = format!("{}{}", &ans[..i], &ans[j..]);
                let (Some(pi), Some(po)) = (self.dict.part(inner), self.dict.part(&outer)) else {
                    continue;
                };
                if pi.letters.len() == 1 && po.letters.len() <= 2 {
                    continue;
                }
                let (ti, to) = (part_text(&pi, rng), part_text(&po, rng));
                let text = if rng.chance(0.5) {
                    format!("{ti} {} {to}", rng.pick(CONTAIN_IN))
                } else {
                    format!("{to} {} {ti}", rng.pick(CONTAIN_OUT))
                };
                let shown = format!("{}({}){}", &ans[..i], inner, &ans[j..]);
                let explain = format!(
                    "{} goes inside {}: {shown}",
                    part_explain(&pi, ti),
                    part_explain(&po, to)
                );
                let score = Self::parts_score(&[pi, po]) + 1.0;
                out.push(Play {
                    device: Device::Container,
                    text,
                    explain,
                    score,
                });
            }
        }
        out
    }

    fn reversals(&self, ans: &str, down: bool, rng: &mut Rng) -> Vec<Play> {
        let rev: String = ans.chars().rev().collect();
        if rev == ans {
            return vec![];
        }
        let inds = if down { REVERSE_DOWN } else { REVERSE_ACROSS };
        let dir = if down { "read upwards" } else { "reversed" };
        let mut out = Vec::new();
        if let Some(p) = self.dict.part(&rev).filter(|p| !p.abbrev) {
            let t = part_text(&p, rng);
            let ind = rng.pick(inds);
            out.push(Play {
                device: Device::Reversal,
                text: format!("{t} {ind}"),
                explain: format!(
                    "{} {dir} (signalled by {}) gives {ans}",
                    part_explain(&p, t),
                    q(ind)
                ),
                score: 6.0,
            });
        }
        for parts in self.splits(&rev, 2) {
            if parts.iter().any(|p| p.letters.len() == 1) {
                continue;
            }
            let t: Vec<&str> = parts.iter().map(|p| part_text(p, rng)).collect();
            let ind = rng.pick(inds);
            let joined = parts
                .iter()
                .zip(&t)
                .map(|(p, t)| part_explain(p, t))
                .collect::<Vec<_>>()
                .join(" + ");
            out.push(Play {
                device: Device::Reversal,
                text: format!("{} and {} {ind}", t[0], t[1]),
                explain: format!(
                    "{joined} = {rev}, {dir} (signalled by {}) gives {ans}",
                    q(ind)
                ),
                score: Self::parts_score(&parts) - 0.5,
            });
        }
        out
    }

    fn deletions(&self, ans: &str, rng: &mut Rng) -> Vec<Play> {
        let d = self.dict;
        let usable = |w: &str| {
            d.get(w)
                .filter(|w| w.freq >= 3.5)
                .and_then(|_| d.part(w))
                .filter(|p| !p.abbrev)
        };
        let mut out = Vec::new();
        for &x in &d.abbrev_keys {
            let Some(px) = d.part(x) else { continue };
            for (whole, pos) in [(format!("{x}{ans}"), "front"), (format!("{ans}{x}"), "end")] {
                let Some(pw) = usable(&whole) else { continue };
                let (tw, tx) = (part_text(&pw, rng), part_text(&px, rng));
                let ind = rng.pick(DELETE_IND);
                out.push(Play {
                    device: Device::Deletion,
                    text: format!("{tw} {ind} {tx}"),
                    explain: format!(
                        "{} with {} taken off the {pos} (signalled by {}) leaves {ans}",
                        part_explain(&pw, tw),
                        part_explain(&px, tx),
                        q(ind)
                    ),
                    score: 3.6,
                });
            }
        }
        for c in b'A'..=b'Z' {
            let c = c as char;
            for (whole, head) in [(format!("{c}{ans}"), true), (format!("{ans}{c}"), false)] {
                let Some(pw) = usable(&whole) else { continue };
                let tw = part_text(&pw, rng);
                let form = if head {
                    rng.pick(BEHEAD)
                } else {
                    rng.pick(CURTAIL)
                };
                let what = if head { "first" } else { "last" };
                out.push(Play {
                    device: Device::Deletion,
                    text: form.replace("{w}", tw),
                    explain: format!(
                        "{} without its {what} letter leaves {ans}",
                        part_explain(&pw, tw)
                    ),
                    score: 3.3,
                });
            }
        }
        out
    }

    fn double_defs(
        &self,
        defs: &[(String, Option<&'static str>, bool)],
        theme: bool,
        rng: &mut Rng,
    ) -> Vec<Cand> {
        let mut out = Vec::new();
        for (i, (a, la, da)) in defs.iter().enumerate() {
            // a theme answer must be clued by its theme definition
            if theme && i > 0 {
                break;
            }
            for (b, lb, db) in defs.iter().skip(i + 1) {
                if !da || !db {
                    continue;
                }
                let distinct = match (la, lb) {
                    (Some(x), Some(y)) => x != y,
                    _ => true, // theme definition vs dictionary sense
                };
                if !distinct || a.contains(b.as_str()) || b.contains(a.as_str()) {
                    continue;
                }
                let long = a.split(' ').count() + b.split(' ').count();
                if long > 9 {
                    continue;
                }
                let (x, y) = if rng.chance(0.5) { (a, b) } else { (b, a) };
                out.push(Cand {
                    device: Device::DoubleDef,
                    text: format!("{} {}", cap(x), y),
                    explain: format!("two separate definitions: {} and {}", q(x), q(y)),
                    score: 5.2 - long as f64 * 0.15,
                });
            }
        }
        out
    }

    fn candidates(
        &self,
        ans: &str,
        down: bool,
        theme: bool,
        defs: &[(String, Option<&'static str>, bool)],
        rng: &mut Rng,
    ) -> Vec<Cand> {
        let mut plays = Vec::new();
        plays.extend(self.anagrams(ans, rng));
        plays.extend(self.hiddens(ans));
        plays.extend(self.charades(ans, down, rng));
        plays.extend(self.containers(ans, rng));
        plays.extend(self.reversals(ans, down, rng));
        plays.extend(self.deletions(ans, rng));

        let mut cands: Vec<Cand> = Vec::new();
        cands.extend(self.and_lits(ans, rng));
        cands.extend(self.double_defs(defs, theme, rng));
        for p in plays {
            // theme answers keep their themed definition; otherwise favour true synonyms
            let def = if theme || defs.len() == 1 {
                defs[0].0.clone()
            } else {
                let w: Vec<f64> = defs
                    .iter()
                    .enumerate()
                    .map(|(i, d)| if d.2 { 3.0 } else { 0.6 } / (1 + i) as f64)
                    .collect();
                defs[rng.weighted(&w)].0.clone()
            };
            let text = match p.device {
                Device::Hidden => match rng.below(3) {
                    0 => format!("{} {} {}", cap(&def), rng.pick(HIDDEN_DEF_FIRST), p.text),
                    1 => join3(
                        &format!("{} {}", rng.pick(HIDDEN_LEAD), p.text),
                        rng.pick_str(LINK_DEF_LAST),
                        &def,
                    ),
                    _ => format!("{} {} {}", cap(&p.text), rng.pick(HIDDEN_VERB), def),
                },
                _ if rng.chance(0.5) => cap(&join3(&def, rng.pick_str(LINK_DEF_FIRST), &p.text)),
                _ => cap(&join3(&p.text, rng.pick_str(LINK_DEF_LAST), &def)),
            };
            let explain = format!("definition {}; {}", q(&def), p.explain);
            cands.push(Cand {
                device: p.device,
                text,
                explain,
                score: p.score,
            });
        }

        // Never print the answer, or a real chunk of it, as a word in the clue
        // ("electronic mail" for EMAIL gives the game away).
        let lower = ans.to_lowercase();
        // real words of 4+ letters inside the answer (USUAL in USUALLY, MAIL in EMAIL)
        let mut chunks = Vec::new();
        for i in 0..ans.len() {
            for j in i + 4..=ans.len() {
                if self.dict.get(&ans[i..j]).is_some() {
                    chunks.push(lower[i..j].to_string());
                }
            }
        }
        cands.retain(|c| {
            !c.text
                .to_lowercase()
                .split(|ch: char| !ch.is_alphabetic())
                .any(|w| {
                    w == lower
                        || (w.len() >= 4
                            && (lower.contains(w) || (lower.len() >= 4 && w[..4] == lower[..4])))
                        || chunks.iter().any(|x| w.contains(x.as_str()))
                })
        });
        cands
    }

    /// Whether an ordinary grid word can get a real cryptic clue.
    pub fn clueable_word(&self, ans: &str) -> bool {
        let Some(w) = self.dict.get(ans) else {
            return false;
        };
        let defs: Vec<_> = w
            .senses
            .iter()
            .filter_map(|s| Some((s.defs.first()?.to_string(), Some(s.lex), *s.direct.first()?)))
            .collect();
        let mut r = Rng::from_str(ans);
        !defs.is_empty() && !self.candidates(ans, false, false, &defs, &mut r).is_empty()
    }

    /// Whether a theme word can get a real cryptic clue (so it's worth putting in the grid).
    pub fn clueable(&self, t: &ThemeEntry) -> bool {
        let defs = vec![(t.def.clone(), None, true)];
        let mut defs = defs;
        if let Some(w) = self.dict.get(&t.answer) {
            for s in &w.senses {
                if let (Some(d), Some(&direct)) = (s.defs.first(), s.direct.first()) {
                    defs.push((d.to_string(), Some(s.lex), direct));
                }
            }
        }
        let mut r = Rng::from_str(&t.answer);
        !self
            .candidates(&t.answer, false, true, &defs, &mut r)
            .is_empty()
    }

    pub fn clue(
        &self,
        entry: &Entry,
        themes: &[ThemeEntry],
        used: &[Device],
        rng: &mut Rng,
    ) -> Clue {
        let ans = entry.answer.as_str();
        let defs = self.defs_for(entry, themes);
        let mut cands = self.candidates(ans, !entry.slot.across, entry.theme.is_some(), &defs, rng);

        let enumeration = entry
            .theme
            .map(|t| themes[t].enumeration.clone())
            .unwrap_or_else(|| format!("({})", ans.len()));

        let chosen = if cands.is_empty() {
            let def = defs
                .first()
                .map(|d| d.0.clone())
                .unwrap_or_else(|| "?".into());
            Cand {
                device: Device::DefinitionOnly,
                text: cap(&def),
                explain:
                    "no fair wordplay found in the word list, so this is a straight definition"
                        .into(),
                score: 0.0,
            }
        } else {
            // best few, with a nudge toward devices the puzzle hasn't used yet
            cands.sort_by(|a, b| b.score.total_cmp(&a.score));
            let weights: Vec<f64> = cands
                .iter()
                .map(|c| {
                    let repeats = used.iter().filter(|&&d| d == c.device).count() as f64;
                    (c.score.max(0.1)).powi(3) / (1.0 + repeats).powi(3)
                })
                .collect();
            let i = rng.weighted(&weights);
            cands.swap_remove(i)
        };

        Clue {
            number: entry.slot.number,
            across: entry.slot.across,
            text: chosen.text,
            enumeration,
            answer: ans.to_string(),
            device: chosen.device,
            explain: format!("{}: {}", chosen.device.name(), chosen.explain),
            theme: entry.theme.is_some(),
        }
    }
}
