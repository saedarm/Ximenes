//! The baked-in word list: definitions for clue parts, plus the lookup indexes
//! the clue devices need. HashMaps here are only ever used for lookups, never
//! iterated, so output order stays deterministic.

use std::collections::HashMap;

const WORDS: &str = include_str!("../data/words.tsv");
const ABBREVS: &str = include_str!("../data/abbrev.tsv");

pub struct Sense {
    pub lex: &'static str,
    pub defs: Vec<&'static str>,
    /// Parallel to `defs`: true for a true synonym, false for a looser
    /// broader term or a short gloss.
    pub direct: Vec<bool>,
}

pub struct Word {
    pub text: &'static str,
    pub freq: f32,
    pub senses: Vec<Sense>,
}

/// Something that can stand for a run of letters in wordplay.
#[derive(Clone)]
pub struct Part {
    pub letters: String,
    pub defs: Vec<&'static str>,
    /// Short enough to be an abbreviation rather than a real word.
    pub abbrev: bool,
}

pub struct Dict {
    pub words: Vec<Word>,
    index: HashMap<&'static str, usize>,
    abbrevs: HashMap<&'static str, Vec<&'static str>>,
    /// Abbreviation keys in file order, for deterministic iteration.
    pub abbrev_keys: Vec<&'static str>,
    anagrams: HashMap<Vec<u8>, Vec<usize>>,
    prefixes: HashMap<&'static str, Vec<usize>>,
    suffixes: HashMap<&'static str, Vec<usize>>,
}

/// Words common enough to appear in a surface reading (anagram fodder, hidden phrases).
pub const SURFACE_FREQ: f32 = 3.3;

pub fn sorted_key(s: &str) -> Vec<u8> {
    let mut k = s.as_bytes().to_vec();
    k.sort_unstable();
    k
}

impl Dict {
    pub fn load() -> Dict {
        let mut words = Vec::new();
        for line in WORDS.trim_start_matches('\u{feff}').lines() {
            let mut cols = line.split('\t');
            let (Some(text), Some(freq)) = (cols.next(), cols.next()) else {
                continue;
            };
            let senses = cols
                .next()
                .unwrap_or("")
                .split(';')
                .filter_map(|s| {
                    let (lex, defs) = s.split_once('=')?;
                    let raw: Vec<&str> = defs.split('|').collect();
                    Some(Sense {
                        lex,
                        direct: raw.iter().map(|d| !d.starts_with(['^', '@'])).collect(),
                        defs: raw
                            .iter()
                            .map(|d| d.trim_start_matches(['^', '@']))
                            .collect(),
                    })
                })
                .collect();
            words.push(Word {
                text,
                freq: freq.parse::<f32>().unwrap_or(0.0) / 10.0,
                senses,
            });
        }

        let mut abbrevs = HashMap::new();
        let mut abbrev_keys = Vec::new();
        for line in ABBREVS
            .trim_start_matches('\u{feff}')
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        {
            if let Some((k, v)) = line.split_once('\t') {
                abbrevs.insert(k, v.split('|').collect());
                abbrev_keys.push(k);
            }
        }

        let mut index = HashMap::new();
        let mut anagrams: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();
        let mut prefixes: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut suffixes: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, w) in words.iter().enumerate() {
            index.insert(w.text, i);
            if w.freq < SURFACE_FREQ {
                continue;
            }
            anagrams.entry(sorted_key(w.text)).or_default().push(i);
            for k in 1..w.text.len() {
                prefixes.entry(&w.text[..k]).or_default().push(i);
                suffixes.entry(&w.text[k..]).or_default().push(i);
            }
        }
        Dict {
            words,
            index,
            abbrevs,
            abbrev_keys,
            anagrams,
            prefixes,
            suffixes,
        }
    }

    pub fn get(&self, s: &str) -> Option<&Word> {
        self.index.get(s).map(|&i| &self.words[i])
    }

    /// Words (3-7 letters) with definitions, i.e. allowed in the grid.
    pub fn fill_words(&self) -> impl Iterator<Item = (usize, &Word)> {
        self.words
            .iter()
            .enumerate()
            .filter(|(_, w)| (3..=7).contains(&w.text.len()) && !w.senses.is_empty())
    }

    /// Everything a run of letters can be clued as inside wordplay.
    pub fn part(&self, s: &str) -> Option<Part> {
        let mut defs: Vec<&'static str> = Vec::new();
        if let Some(a) = self.abbrevs.get(s) {
            defs.extend(a.iter().copied());
        }
        if let Some(w) = self.get(s)
            && w.freq >= 3.0
        {
            for sense in w.senses.iter().take(2) {
                if let Some(d) = sense.defs.first()
                    && !defs.contains(d)
                    && d.split(' ').count() <= 2
                {
                    defs.push(d);
                }
            }
        }
        if defs.is_empty() {
            return None;
        }
        Some(Part {
            letters: s.to_string(),
            defs,
            abbrev: s.len() <= 2,
        })
    }

    pub fn anagrams_of(&self, key: &[u8]) -> &[usize] {
        self.anagrams.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Surface words that start with `p` and are longer than it.
    pub fn with_prefix(&self, p: &str) -> &[usize] {
        self.prefixes.get(p).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Surface words that end with `s` and are longer than it.
    pub fn with_suffix(&self, s: &str) -> &[usize] {
        self.suffixes.get(s).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn is_surface_word(&self, s: &str) -> bool {
        self.get(s).is_some_and(|w| w.freq >= SURFACE_FREQ)
    }
}
