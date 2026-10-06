# Ximenes

A command-line tool that builds a themed 7x7 cryptic crossword and then explains every clue.

You give it a list of theme words. It fills a 7x7 grid with as many of them as it can fit, writes a cryptic clue for every answer, and prints (or exports to PDF/SVG) the puzzle, the solution, and a line for each clue saying exactly how it works.

```
$ ximenes new --theme space --seed launch --reveal

ACROSS
  1  Icy visitor with a tail baby bed holding me (5)
  4  Partly flu narrator gives of the moon (5)
  ...

SOLUTION
 1a  COMET *theme
     Container: definition 'icy visitor with a tail'; ME (as is) goes inside COT ('baby bed'): CO(ME)T
 4a  LUNAR *theme
     Hidden word: definition 'of the moon'; LUNAR is hidden in 'fLU NARrator'
```

The name comes from Ximenes, the pen name of D.S. Macnutt, whose rules for fair cryptic clues are the ones this tool follows.

## Install

You need Rust 1.88 or newer ([rustup.rs](https://rustup.rs)).

From the source folder:

```
cargo install --path .
```

That puts `ximenes` on your PATH. Or skip installing and run it through Cargo with `cargo run --release -- <command>`.

## Try it

Built-in themes:

| theme | what's in it |
|---|---|
| `kitchen` | oven, whisk, ladle, kettle, skillet... |
| `space` | orbit, comet, nebula, galaxy, eclipse... |
| `ocean` | tide, coral, shark, oyster, lagoon... |
| `weather` | sleet, storm, thunder, breeze, tornado... |
| `music` | piano, cello, tempo, chord, opera... |
| `market-garden` | places and people from Operation Market Garden (1944) |
| `cosmic-encounter` | aliens from the board game Cosmic Encounter |

Some commands to start with:

```
ximenes themes                                     # list the built-in themes
ximenes new                                        # kitchen theme, random seed
ximenes new --theme ocean --reveal                 # show the answers and explanations
ximenes new --theme space --seed launch --reveal   # a fixed seed: same puzzle every time
ximenes new --theme music --pdf music.pdf          # printable PDF with a solution page
ximenes new --theme weather --svg weather.svg --answers
ximenes open 1.space.launch --reveal               # rebuild a puzzle from its share code
ximenes new --theme samples/my-theme.txt --reveal  # your own word list
ximenes new --theme samples/phrases.txt --reveal   # answers with two words, like POP STAR (3,4)
```

The `samples/` folder has two custom theme files to copy (`my-theme.txt` and `phrases.txt`) and example PDF and SVG output.

## Things to test

If you're kicking the tires, these are the behaviours worth checking:

1. **Same seed, same puzzle.** Run `ximenes new --theme ocean --seed reef` twice. The output should match exactly, every character.
2. **Share codes rebuild puzzles.** Copy the share code from any puzzle and run `ximenes open <code>`. You should get the same grid and clues back.
3. **Custom codes carry their own word list.** Run `ximenes new --theme samples/my-theme.txt`, delete or rename the file, then `ximenes open` the code it printed. It still works, because the word list is packed inside the code.
4. **Different seeds, different puzzles.** `--seed a` and `--seed b` on the same theme should give different grids.
5. **Every clue explains itself.** Add `--reveal` and check any explanation against its clue. The pieces it names should add up to the answer.
6. **Bad input gets a clear message.** A word over 7 letters is skipped with a note. A line with no `=` stops with the line number. An unknown theme name says so.
7. **Printing.** `--pdf puzzle.pdf` should give two pages, the puzzle and then the solution with explanations.

## Commands and options

```
ximenes new   [--theme <name or file>] [--seed <text>] [output options]
ximenes open  <share code>                             [output options]
ximenes themes
```

| option | what it does |
|---|---|
| `-t, --theme` | built-in theme name, or a path to your own word list (default: `kitchen`) |
| `-s, --seed` | any text; the same theme and seed always give the same puzzle (default: random) |
| `-r, --reveal` | also print the solution grid and how each clue works |
| `--pdf FILE` | two-page PDF: puzzle, then solution with explanations |
| `--svg FILE` | printable SVG of the puzzle |
| `--answers` | add the solution page to the SVG |

## Writing your own theme

A theme is a text file with one entry per line, `WORD = definition`. A `#` line at the top is the puzzle's title.

```
# Game Night
DICE = they're rolled
TOKEN = game piece
CHESS = game of kings
POKER = card game with bluffing
RULES = what the box insert explains
```

- Answers must be 3 to 7 letters. Spaces and hyphens are allowed and show up in the enumeration, e.g. `BIG CAT` gives `(3,3)`.
- The definition is used word for word in the clue, so write it the way a crossword would: short, and something the answer could stand in for in a sentence.
- 10 to 20 words is a good size. The grid only holds 8 to 12 answers, and usually 3 to 5 of them end up being theme words.
- If a theme word can't get any fair wordplay from the dictionary, Ximenes leaves it out and prints a note saying so.

## Share codes

Every puzzle prints a share code like `1.space.launch`. Anyone running the same version of Ximenes can run `ximenes open 1.space.launch` and get the identical puzzle, clues and explanations included.

For a custom word list, the list itself is compressed into the code, so the code works without the file. A 13-word theme makes a code of about 300 characters.

The `1.` at the start is the format version. If a future change to the dictionary or the generator would make old codes produce different puzzles, that number goes up and old codes give a clear error instead of a different puzzle.

## How the clues work

Every cryptic clue has a definition (at the start or end) and wordplay that builds the same answer another way. Ximenes uses these devices. Every example below is real output, and the last column is the theme and seed that produce it, so you can run `ximenes new --theme <theme> --seed <seed> --reveal` and see it yourself.

| device | example | how it works | theme, seed |
|---|---|---|---|
| Anagram | *New tan door twister (7)* | TAN + DOOR scrambled ("new") = TORNADO | weather, s9 |
| Hidden word | *Tempest in pastor me (5)* | paSTOR Me = STORM | weather, s13 |
| Charade | *Cover above year for foggy (5)* | MIST (cover) + Y (year) = MISTY | weather, s15 |
| Container | *Hot kept by sale beach find (5)* | H (hot) inside SELL (sale) = SHELL | ocean, s8 |
| Reversal | *Kitchen alarm from issue up (5)* | REMIT (issue) read upwards = TIMER | kitchen, s29 |
| Deletion | *Go minus note sung drama (5)* | OPERATE (go) minus TE (note) = OPERA | music, s6 |
| Double definition | *Mist cloud (3)* | two meanings of FOG | weather, s26 |
| &lit | e.g. *Angered madly! (7)* | the whole clue is both definition and wordplay: ENRAGED | rare, see below |

These examples only reproduce with this version of the dictionary and generator. If either changes, the clues for a given seed change with them.

&lit clues only appear when the dictionary has a genuine one, an anagram whose letters are themselves a synonym of the answer (ANGERED/ENRAGED, SHAPE/PHASE, SPOT/POST, TONE/NOTE). That's about a dozen words, so they're rare.

Ximenes also checks every clue for fairness. The answer never appears in its own clue, and neither does any real word of 4 or more letters hidden inside the answer.

## How it works

- **Grid** (`src/grid.rs`): starts from the classic cryptic lattice, where every other square on every other row is black, then adds up to three mirrored pairs of extra black squares. It pins a few theme words into the grid, then fills the rest by backtracking, always working on the slot with the fewest possible words first. It tries 400 grid shapes and keeps the one with the most theme words.
- **Clues** (`src/clue.rs`): for each answer, tries every device against the dictionary, scores each candidate, and picks one, favouring devices the puzzle hasn't used much yet.
- **Dictionary** (`src/dict.rs`): about 16,000 words with definitions, baked into the program at compile time, plus about 110 standard crossword abbreviations.
- **Themes and share codes** (`src/theme.rs`), **random numbers** (`src/rng.rs`), **text/SVG/PDF output** (`src/render.rs`).

The random number generator is written into the program (FNV-1a to hash the seed, SplitMix64 to generate) rather than taken from a library, so a library update can never change what a share code produces. The search is limited by step count, not time, so a seed gives the same puzzle on a fast machine and a slow one.

## Building from source

For a step-by-step guide to building the project from an empty folder, see [BUILDING.md](BUILDING.md).

```
cargo build --release      # binary ends up in target/release/
cargo test --release       # includes a check that share codes rebuild identical puzzles
cargo clippy --all-targets
cargo fmt --check
```

The only dependencies are `clap` (command-line parsing) and `miniz_oxide` (compression for share codes). The PDF writer, SVG writer and base64 encoding are written by hand.

### Regenerating the dictionary

`data/words.tsv` is built from WordNet and wordfreq by `tools/build_dict.py`. You don't need Python to build or run Ximenes, only to rebuild that file:

```
pip install nltk wordfreq
python -c "import nltk; nltk.download('wordnet')"
python tools/build_dict.py > data/words.tsv
```

Changing `words.tsv` changes the puzzles that existing share codes produce, so bump the code version in `src/theme.rs` if you publish a new dictionary.

## Project layout

```
Cargo.toml
src/
  main.rs      command line, puzzle assembly
  grid.rs      grid shapes and the backtracking filler
  clue.rs      clue devices, scoring, explanations
  dict.rs      dictionary loading and lookups
  theme.rs     theme files and share codes
  rng.rs       seeded random numbers
  render.rs    text, SVG and PDF output
data/
  words.tsv    the dictionary (generated)
  abbrev.tsv   crossword abbreviations
  LICENSE-DATA.md
themes/        the built-in themes
samples/       example theme files and example output
BUILDING.md    building the project from scratch, step by step
tools/
  build_dict.py
```

## Limits

- The clue sentences read like a program wrote them. The wordplay is always sound, but a clue like *Partly flu narrator gives of the moon* is fair and still clumsy. Treat the output as a first draft if you want to polish it by hand.
- WordNet sometimes reaches for an unusual sense of a word, so a definition can be correct but dated.
- Very occasionally (about 1 clue in 200 in testing), no fair wordplay exists for an ordinary grid word and the clue is a straight definition. The explanation says so when it happens.
- A 7x7 grid is small. Expect 3 to 5 theme words per puzzle.

## License

The code is MIT licensed (see `LICENSE`).

The dictionary in `data/words.tsv` is derived from WordNet 3.0 (Copyright 2006 by Princeton University) and from wordfreq's word-frequency data (CC BY-SA 4.0), so that file is released under CC BY-SA 4.0. Full details and the required WordNet notice are in `data/LICENSE-DATA.md`.
