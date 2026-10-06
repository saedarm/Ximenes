//! 7x7 grid templates and the backtracking filler.
//!
//! Templates start from the classic cryptic lattice (every odd/odd cell black,
//! so roughly half of each entry's letters are checked) and then add a few
//! extra blocks in 180-degree-symmetric pairs to break long entries up.

use crate::dict::Dict;
use crate::rng::Rng;
use crate::theme::ThemeEntry;

pub const N: usize = 7;
pub type Blocks = [[bool; N]; N];

#[derive(Clone)]
pub struct Slot {
    pub number: usize,
    pub across: bool,
    pub cells: Vec<(usize, usize)>,
}

#[derive(Clone)]
pub struct Entry {
    pub slot: Slot,
    pub answer: String,
    /// Index into the theme list when this entry is a theme word.
    pub theme: Option<usize>,
}

pub struct Fill {
    pub blocks: Blocks,
    pub letters: [[u8; N]; N],
    pub entries: Vec<Entry>,
}

impl Fill {
    pub fn theme_count(&self) -> usize {
        self.entries.iter().filter(|e| e.theme.is_some()).count()
    }
}

fn lattice() -> Blocks {
    let mut b = [[false; N]; N];
    for r in (1..N).step_by(2) {
        for c in (1..N).step_by(2) {
            b[r][c] = true;
        }
    }
    b
}

pub fn template(rng: &mut Rng) -> Blocks {
    // Cells that sit between two lattice blocks; blocking one splits an entry.
    let mut spots = Vec::new();
    for r in 0..N {
        for c in 0..N {
            if (r + c) % 2 == 1 && (r, c) <= (N - 1 - r, N - 1 - c) {
                spots.push((r, c));
            }
        }
    }
    for _ in 0..500 {
        let mut b = lattice();
        let pairs = rng.weighted(&[2.0, 4.0, 4.0, 2.0]);
        for _ in 0..pairs {
            let &(r, c) = rng.pick(&spots);
            b[r][c] = true;
            b[N - 1 - r][N - 1 - c] = true;
        }
        if slots_of(&b).is_some() {
            return b;
        }
    }
    lattice()
}

/// Numbered entries for a block pattern, or None if the pattern is unusable
/// (two-letter runs, orphaned cells, or a disconnected grid).
pub fn slots_of(b: &Blocks) -> Option<Vec<Slot>> {
    let white = |r: usize, c: usize| !b[r][c];
    let mut covered = [[false; N]; N];
    let mut starts: Vec<(usize, usize, bool, usize)> = Vec::new();

    for across in [true, false] {
        for line in 0..N {
            let mut i = 0;
            while i < N {
                let (r, c) = if across { (line, i) } else { (i, line) };
                if !white(r, c) {
                    i += 1;
                    continue;
                }
                let start = i;
                while i < N && {
                    let (r, c) = if across { (line, i) } else { (i, line) };
                    white(r, c)
                } {
                    i += 1;
                }
                let len = i - start;
                if len == 2 {
                    return None;
                }
                if len >= 3 {
                    let (r, c) = if across { (line, start) } else { (start, line) };
                    starts.push((r, c, across, len));
                    for k in start..i {
                        let (r, c) = if across { (line, k) } else { (k, line) };
                        covered[r][c] = true;
                    }
                }
            }
        }
    }

    // every white cell used, and all white cells connected
    let mut seen = [[false; N]; N];
    let mut stack = Vec::new();
    let mut whites = 0;
    for r in 0..N {
        for c in 0..N {
            if white(r, c) {
                whites += 1;
                if !covered[r][c] {
                    return None;
                }
                if stack.is_empty() && !seen.iter().flatten().any(|&s| s) {
                    stack.push((r, c));
                    seen[r][c] = true;
                }
            }
        }
    }
    let mut reached = 0;
    while let Some((r, c)) = stack.pop() {
        reached += 1;
        let nbrs = [
            (r.wrapping_sub(1), c),
            (r + 1, c),
            (r, c.wrapping_sub(1)),
            (r, c + 1),
        ];
        for (nr, nc) in nbrs {
            if nr < N && nc < N && white(nr, nc) && !seen[nr][nc] {
                seen[nr][nc] = true;
                stack.push((nr, nc));
            }
        }
    }
    if reached != whites {
        return None;
    }

    // number in reading order
    starts.sort_by_key(|&(r, c, across, _)| (r, c, !across));
    let mut slots = Vec::new();
    let mut number = 0;
    let mut last = None;
    for (r, c, across, len) in starts {
        if last != Some((r, c)) {
            number += 1;
            last = Some((r, c));
        }
        let cells = (0..len)
            .map(|k| if across { (r, c + k) } else { (r + k, c) })
            .collect();
        slots.push(Slot {
            number,
            across,
            cells,
        });
    }
    Some(slots)
}

struct Cand {
    text: Vec<u8>,
    theme: Option<usize>,
}

/// Candidate words of one length, ordered best-first, with a bitset per
/// (position, letter) so pattern matching is a handful of ANDs.
struct Pool {
    words: Vec<Cand>,
    bits: Vec<Vec<Vec<u64>>>, // [pos][letter] -> bitset over words
    blocks: usize,
}

impl Pool {
    fn new(words: Vec<Cand>, len: usize) -> Pool {
        let blocks = words.len().div_ceil(64);
        let mut bits = vec![vec![vec![0u64; blocks]; 26]; len];
        for (i, w) in words.iter().enumerate() {
            for (p, &ch) in w.text.iter().enumerate() {
                bits[p][(ch - b'A') as usize][i / 64] |= 1 << (i % 64);
            }
        }
        Pool {
            words,
            bits,
            blocks,
        }
    }

    fn matching(&self, pattern: &[u8]) -> Vec<u64> {
        let mut acc = vec![!0u64; self.blocks];
        if let Some(last) = acc.last_mut() {
            let tail = self.words.len() % 64;
            if tail != 0 {
                *last = (1u64 << tail) - 1;
            }
        }
        for (p, &ch) in pattern.iter().enumerate() {
            if ch != 0 {
                for (a, b) in acc.iter_mut().zip(&self.bits[p][(ch - b'A') as usize]) {
                    *a &= b;
                }
            }
        }
        acc
    }
}

struct Solver<'a> {
    slots: &'a [Slot],
    pools: &'a [Option<Pool>; N + 1],
    letters: [[u8; N]; N],
    chosen: Vec<Option<usize>>,
    used: Vec<Vec<u8>>,
    nodes: usize,
    budget: usize,
}

impl Solver<'_> {
    fn pattern(&self, s: &Slot) -> Vec<u8> {
        s.cells.iter().map(|&(r, c)| self.letters[r][c]).collect()
    }

    fn fits(&self, si: usize, text: &[u8]) -> bool {
        self.chosen[si].is_none()
            && self.slots[si].cells.len() == text.len()
            && self
                .pattern(&self.slots[si])
                .iter()
                .zip(text)
                .all(|(&p, &t)| p == 0 || p == t)
    }

    fn place(&mut self, si: usize, wi: usize) -> Vec<(usize, usize)> {
        let text = self.pools[self.slots[si].cells.len()]
            .as_ref()
            .unwrap()
            .words[wi]
            .text
            .clone();
        let mut placed = Vec::new();
        for (k, &(r, c)) in self.slots[si].cells.iter().enumerate() {
            if self.letters[r][c] == 0 {
                self.letters[r][c] = text[k];
                placed.push((r, c));
            }
        }
        self.chosen[si] = Some(wi);
        self.used.push(text);
        placed
    }

    fn solve(&mut self) -> bool {
        if self.nodes >= self.budget {
            return false;
        }
        self.nodes += 1;
        // most-constrained open slot first
        let mut best: Option<(usize, usize, Vec<u64>)> = None;
        for (i, s) in self.slots.iter().enumerate() {
            if self.chosen[i].is_some() {
                continue;
            }
            let pool = self.pools[s.cells.len()].as_ref().unwrap();
            let bits = pool.matching(&self.pattern(s));
            let count: usize = bits.iter().map(|b| b.count_ones() as usize).sum();
            if count == 0 {
                return false;
            }
            if best.as_ref().is_none_or(|(_, c, _)| count < *c) {
                best = Some((i, count, bits));
            }
        }
        let Some((si, _, bits)) = best else {
            return true;
        };
        let pool = self.pools[self.slots[si].cells.len()].as_ref().unwrap();

        for (bi, &block) in bits.iter().enumerate() {
            let mut b = block;
            while b != 0 {
                let wi = bi * 64 + b.trailing_zeros() as usize;
                b &= b - 1;
                if self.used.contains(&pool.words[wi].text) {
                    continue;
                }
                let placed = self.place(si, wi);
                if self.solve() {
                    return true;
                }
                self.used.pop();
                self.chosen[si] = None;
                for (r, c) in placed {
                    self.letters[r][c] = 0;
                }
                if self.nodes >= self.budget {
                    return false;
                }
            }
        }
        false
    }
}

/// Tries several templates and keeps the fill that uses the most theme words.
/// Work is bounded by node counts, not wall-clock time, so a seed always
/// produces the same grid on any machine.
/// `clueable` reports whether an ordinary word can get real wordplay; fills
/// with words that can't are marked down.
pub fn fill(
    dict: &Dict,
    themes: &[ThemeEntry],
    rng: &mut Rng,
    attempts: usize,
    clueable: &mut dyn FnMut(&str) -> bool,
) -> Option<Fill> {
    let mut best: Option<(f64, Fill)> = None;
    for attempt in 0..attempts {
        let mut r = rng.fork(&format!("attempt{attempt}"));
        let blocks = template(&mut r);
        let slots = slots_of(&blocks).expect("template is valid");

        let mut pools: [Option<Pool>; N + 1] = Default::default();
        #[allow(clippy::needless_range_loop)] // `len` is a word length, not just an index
        for len in 3..=N {
            if !slots.iter().any(|s| s.cells.len() == len) {
                continue;
            }
            let mut scored: Vec<(f64, Cand)> = Vec::new();
            for (ti, t) in themes.iter().enumerate() {
                if t.answer.len() == len {
                    scored.push((
                        100.0 + r.unit(),
                        Cand {
                            text: t.answer.as_bytes().to_vec(),
                            theme: Some(ti),
                        },
                    ));
                }
            }
            for (_, w) in dict.fill_words() {
                if w.text.len() == len && !themes.iter().any(|t| t.answer == w.text) {
                    // common words first, with jitter so seeds differ
                    let p = w.freq as f64 + r.unit() * 1.5;
                    scored.push((
                        p,
                        Cand {
                            text: w.text.as_bytes().to_vec(),
                            theme: None,
                        },
                    ));
                }
            }
            scored.sort_by(|a, b| b.0.total_cmp(&a.0));
            pools[len] = Some(Pool::new(scored.into_iter().map(|(_, c)| c).collect(), len));
        }

        let mut solver = Solver {
            slots: &slots,
            pools: &pools,
            letters: [[0; N]; N],
            chosen: vec![None; slots.len()],
            used: Vec::new(),
            nodes: 0,
            budget: 8000,
        };
        // Pin a few theme words into random slots first; the search then has
        // to build the rest of the grid around them.
        let mut order: Vec<usize> = (0..themes.len()).collect();
        for i in (1..order.len()).rev() {
            order.swap(i, r.below(i + 1));
        }
        let anchors = 2 + r.below(5);
        let mut pinned = 0;
        for ti in order {
            if pinned == anchors {
                break;
            }
            let text = themes[ti].answer.as_bytes();
            let Some(pool) = pools[text.len()].as_ref() else {
                continue;
            };
            let Some(wi) = pool.words.iter().position(|w| w.theme == Some(ti)) else {
                continue;
            };
            let open: Vec<usize> = (0..slots.len())
                .filter(|&si| solver.fits(si, text))
                .collect();
            if open.is_empty() {
                continue;
            }
            let si = open[r.below(open.len())];
            solver.place(si, wi);
            pinned += 1;
        }
        if !solver.solve() {
            continue;
        }
        let entries: Vec<Entry> = slots
            .iter()
            .zip(&solver.chosen)
            .map(|(s, &wi)| {
                let c = &pools[s.cells.len()].as_ref().unwrap().words[wi.unwrap()];
                Entry {
                    slot: s.clone(),
                    answer: String::from_utf8(c.text.clone()).unwrap(),
                    theme: c.theme,
                }
            })
            .collect();
        let themed = entries.iter().filter(|e| e.theme.is_some()).count() as f64;
        let freq: f64 = entries
            .iter()
            .map(|e| dict.get(&e.answer).map_or(4.0, |w| w.freq as f64))
            .sum::<f64>()
            / entries.len() as f64;
        let shorts = entries.iter().filter(|e| e.answer.len() == 3).count() as f64;
        let score = themed * 10.0 + freq - shorts * 0.6;
        if best.as_ref().is_some_and(|(s, _)| score <= *s) {
            continue;
        }
        let bare = entries
            .iter()
            .filter(|e| e.theme.is_none() && !clueable(&e.answer))
            .count() as f64;
        let score = score - bare * 15.0;
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((
                score,
                Fill {
                    blocks,
                    letters: solver.letters,
                    entries,
                },
            ));
        }
    }
    best.map(|(_, f)| f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_has_eight_full_entries() {
        let s = slots_of(&lattice()).unwrap();
        assert_eq!(s.len(), 8);
        assert!(s.iter().all(|s| s.cells.len() == 7));
    }

    #[test]
    fn templates_are_symmetric_and_valid() {
        let mut r = Rng::from_str("t");
        for _ in 0..200 {
            let b = template(&mut r);
            for i in 0..N {
                for j in 0..N {
                    assert_eq!(b[i][j], b[N - 1 - i][N - 1 - j]);
                }
            }
            assert!(slots_of(&b).is_some());
        }
    }
}
