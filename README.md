# Ximenes

Named after Ximenes, the pen name of D.S. Macnutt, whose rules for fair cryptic clues are the ones this tool follows.

A Rust CLI that builds a themed 7x7 cryptic crossword and then explains every clue.

You give it a theme word list. It fills a 7x7 grid by backtracking search, trying to fit as many theme words as it can, then writes a cryptic clue for each entry and records exactly how the clue works.

```
$ ximenes new --theme market-garden --seed bridge --reveal

  2  Half roster conceals colonel who held the Arnhem bridge (5)
 ...
 2d  FROST *theme
     Hidden word: definition 'colonel who held the Arnhem bridge'; FROST is hidden in 'halF ROSTer'
```

## Usage

```
ximenes themes                                  # list built-in themes
ximenes new --theme dayton                      # random seed
ximenes new --theme dayton --seed gem-city      # fixed seed
ximenes new --theme my-list.txt --pdf out.pdf   # your own word list
ximenes open 1.dayton.gem-city --reveal         # rebuild from a share code
```

Output flags (on `new` and `open`):

| flag | what it does |
|---|---|
| `--reveal` | print the solution grid and the explanation for every clue |
| `--pdf FILE` | two-page PDF: puzzle, then solution with explanations |
| `--svg FILE` | printable SVG of the puzzle |
| `--answers` | add the solution page to the SVG |

Built-in themes: `market-garden`, `cosmic-encounter`, `dayton`.

## Theme files

One entry per line, `WORD = definition`. A `#` line at the top is the title.

```
# Pizza night
CRUST = edge of the pie
OLIVE = black topping
BIG CHEESE = boss
```

Entries must be 3-7 letters (spaces and hyphens are allowed and become the enumeration, e.g. `(3,6)` — though a 9-letter phrase won't fit a 7x7). The definition is used as-is in the clue, so write it the way a setter would: short, and something the answer could stand in for.

A theme word that can't get any fair wordplay from the word list is left out, with a note on stderr.

## Share codes

Every puzzle prints a share code like `1.dayton.gem-city`. Anyone with the same version of ximenes can run `ximenes open <code>` and get the identical puzzle, clues and explanations included.

For a custom word list, the list itself is compressed into the code (`1.~RY5BTsMw…​.friday`), so the code works without the file. A 10-word list makes a code of about 270 characters.

Determinism comes from three things: a hand-rolled PRNG (no crate upgrade can change it), a search bounded by node count rather than time, and no iteration over hash maps anywhere that affects output. The leading `1.` is the format version; if the word list or generator changes in a way that alters output, that number should go up.

## Clue devices

Each device builds its wordplay from the baked-in word list, so the explanation is an account of what was actually done.

| device | example from real output |
|---|---|
| Anagram | *Bay me dancing perhaps (5)* — MAYBE |
| Hidden word | *Public end conceals crystal (3)* — ICE |
| Charade | *An after noise for alien that adds four to its total (5)* — HUM + AN |
| Container | *Put off from notice swallowed by time period (5)* — EV(AD)E |
| Reversal | a word read backwards; *up* / *rising* in down clues, *back* / *returned* across |
| Deletion | *Grave losing book house cat (3)* — TOMB minus B |
| Double definition | *Deal set (3)* — LOT |
| &lit | e.g. *Angered madly! (7)* — ENRAGED |

&lit clues only appear when the word list has a genuine one: an anagram whose fodder is itself a synonym of the answer (ANGERED/ENRAGED, SHAPE/PHASE, SPOT/POST, TONE/NOTE, PAT/TAP, OUTLOOK/LOOKOUT). That is roughly a dozen words, so they are rare.

Clue selection favours stronger devices and nudges away from ones the puzzle has already used, so a grid doesn't come out as ten hidden words.

## How it works

- **`grid.rs`** — templates start from the classic cryptic lattice (every odd row/odd column cell black, so about half of each entry's letters are checked), then add up to three 180°-symmetric pairs of extra blocks. The filler pins a few theme words into random slots, then fills the rest most-constrained-slot-first, with bitset indexes per (position, letter). It tries 400 templates and keeps the fill with the most theme words, penalising 3-letter entries and any word it couldn't clue.
- **`clue.rs`** — the devices, the indicator lists, and the checks that keep clues fair (the answer, or a 4+ letter word inside it, never appears in the clue text).
- **`dict.rs`** — the word list, abbreviations, and lookup indexes (sorted-letter keys for anagrams, prefix/suffix maps for hidden words).
- **`theme.rs`** — theme parsing and share codes.
- **`render.rs`** — terminal text, SVG, and a dependency-free PDF writer using standard Helvetica.

Dependencies: `clap` and `miniz_oxide`.

## The word list

`data/words.tsv` is generated from WordNet and wordfreq by `tools/build_dict.py` and baked into the binary. You only need Python to regenerate it:

```
pip install nltk wordfreq
python -c "import nltk; nltk.download('wordnet')"
python tools/build_dict.py > data/words.tsv
```

Definitions are WordNet synonyms first, then broader terms (never used for double definitions), then short glosses. `data/abbrev.tsv` holds the usual crossword abbreviations (R = right, E = east, AB = sailor, and so on).

## Limits

- Surfaces are mechanical. The wordplay is always sound, but a clue like *Stack be wildly producing black eye* reads like what it is: a program. Treat the output as a solid first draft if you want to polish it by hand.
- WordNet's sense of a word is sometimes not the one a person would reach for (CLAY for REMAINS is legitimate but old-fashioned).
- A 7x7 lattice holds 8-12 entries, and typically 3-5 of them are theme words.
