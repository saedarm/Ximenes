# Building Ximenes from scratch

This guide takes you from an empty folder to a working, tested, published Ximenes. The commands are written for Windows PowerShell. Where a Mac or Linux terminal needs something different, it's noted.

You can follow it two ways. You can copy each finished file from the source in the order below, reading it as you go. Or you can type each file yourself. Either way, keep to the order. Each step builds on the one before, and there's a check at the end of each step so a mistake shows up right away instead of ten files later.

## 1. Install the tools

**Rust.** Go to [rustup.rs](https://rustup.rs) and run the installer. On Windows it will offer to install the Visual Studio C++ Build Tools. Say yes. Rust uses Microsoft's linker to produce `.exe` files and won't build anything without it.

Open a new terminal and check the versions:

```
rustc --version
cargo --version
```

You need Rust **1.88 or newer**. If yours is older, run `rustup update`.

**Git**, from [git-scm.com](https://git-scm.com), for version control and pushing to GitHub.

**An editor.** RustRover works well and picks up your Rust install on its own. VS Code with the rust-analyzer extension works too.

**Python** is only needed if you want to rebuild the dictionary (step 13). You can skip it otherwise.

## 2. Create the project

```
cargo new ximenes
cd ximenes
```

That gives you:

```
ximenes/
  Cargo.toml      the project's settings and dependencies
  src/main.rs     a "Hello, world!" program
  .gitignore      ignores the target/ build folder (you'll add more in step 10)
  .git/           a fresh Git repository
```

Keep the name lowercase. Cargo package names are case-sensitive, and asking for `Ximenes` gives the error `package ID specification 'Ximenes' did not match any packages`.

Check it works:

```
cargo run
```

You should see `Hello, world!`.

## 3. Fill in Cargo.toml

Replace the contents of `Cargo.toml` with:

```toml
[package]
name = "ximenes"
version = "0.1.0"
edition = "2024"
rust-version = "1.88"
description = "Builds a themed 7x7 cryptic crossword from a seed, then explains every clue"
license = "MIT AND CC-BY-SA-4.0"
repository = "https://github.com/saedarm/ximenes"
readme = "README.md"
keywords = ["crossword", "cryptic", "puzzle", "wordplay", "cli"]
categories = ["command-line-utilities", "games"]
exclude = ["samples/*.pdf", "samples/*.svg"]

[dependencies]
clap = { version = "4", features = ["derive"] }
miniz_oxide = "0.9"

[profile.release]
opt-level = 3
```

What the parts do:

- `edition = "2024"` is the version of the Rust language rules the code follows. `rust-version` is the oldest compiler that can build it. The code uses `if let ... && ...` chains, which arrived in 1.88.
- `license`, `description`, `repository` and the rest are only needed for publishing (step 12), but it's easier to have them from the start.
- `exclude` keeps the sample PDFs out of the published package. They're useful in the GitHub repo but not in a download.
- The two dependencies: `clap` turns command-line arguments into values, and `miniz_oxide` compresses custom word lists into share codes.

**Check:**

```
cargo build
```

Cargo downloads clap and miniz_oxide, builds them, and creates `Cargo.lock`. That file records the exact version of every dependency. Keep it and commit it. For a program (as opposed to a library), it's what guarantees everyone builds with the same dependencies.

## 4. Add the folders and data files

```
mkdir data, themes, samples, tools
```

(That's PowerShell. On Mac or Linux it's `mkdir data themes samples tools`, with no commas.)

Then copy these files in from the source:

| file | what it is |
|---|---|
| `data/words.tsv` | the dictionary: about 16,000 words with definitions (generated, don't edit by hand) |
| `data/abbrev.tsv` | about 110 crossword abbreviations, like `AB = sailor` |
| `data/LICENSE-DATA.md` | the licenses for the dictionary data |
| `themes/*.txt` | the 7 built-in themes |
| `samples/my-theme.txt`, `samples/phrases.txt` | example custom themes |
| `tools/build_dict.py` | the script that generates `words.tsv` |
| `LICENSE` | the MIT license for the code |
| `README.md` | the readme |

Get the paths exactly right. The program bakes `data/` and `themes/` into the executable when it compiles, using paths written into the source code. If a file is missing or misnamed, the build fails with `couldn't read ... data/words.tsv`.

## 5. Add the source files one at a time

The source is seven files. Each one uses the ones before it, so add them in this order:

| order | file | what it does | uses |
|---|---|---|---|
| 1 | `rng.rs` | seeded random numbers (FNV-1a hash + SplitMix64) | nothing |
| 2 | `dict.rs` | loads the dictionary, anagram and prefix/suffix lookups | the data files |
| 3 | `theme.rs` | reads theme files, makes and decodes share codes | the theme files, miniz_oxide |
| 4 | `grid.rs` | grid shapes and the backtracking filler | rng, dict, theme |
| 5 | `clue.rs` | the clue devices, scoring and explanations | rng, dict, theme, grid |
| 6 | `render.rs` | terminal text, SVG and PDF output | grid, clue |
| 7 | `main.rs` | the command line, ties it all together | everything |

Start by replacing `src/main.rs` with a placeholder that only knows about the first file:

```rust
mod rng;

fn main() {}
```

Add `src/rng.rs`, then check it compiles:

```
cargo check
```

`cargo check` compiles without producing a program, so it's much faster than a full build. Then for each of the next files, add the file and one more `mod` line to the top of `main.rs`, and run `cargo check` again:

```rust
mod rng;
mod dict;   // add with dict.rs
mod theme;  // add with theme.rs
mod grid;   // add with grid.rs
mod clue;   // add with clue.rs
mod render; // add with render.rs

fn main() {}
```

You'll see the number of warnings climb with each file, up to about 90, mostly saying things like ``function `load` is never used``. That's expected. Nothing calls this code yet, because the placeholder `main` is empty. They all go away in the next step.

What you're watching for is **errors**, not warnings. If `cargo check` reports an error, fix it before adding the next file.

## 6. Add the real main.rs

Replace the placeholder `src/main.rs` with the real one, then build an optimized version:

```
cargo build --release
```

This time there should be **zero warnings**. The finished program is at `target/release/ximenes.exe` (about 2.3 MB, mostly the dictionary).

## 7. First run

Everything after the `--` goes to Ximenes instead of to Cargo:

```
cargo run --release -- themes
cargo run --release -- new --theme kitchen --seed first-run --reveal
```

The second command should print a grid, the clues, a solution and an explanation for every clue. It should also print `share code: 1.kitchen.first-run`, and `4 theme words in the grid` at the end.

Now try the rest:

```
cargo run --release -- new --theme ocean --reveal
cargo run --release -- new --theme samples/phrases.txt --reveal
cargo run --release -- new --theme space --seed launch --pdf space.pdf
cargo run --release -- open 1.kitchen.first-run
```

The last one should print exactly the same puzzle as the `first-run` command above.

In RustRover, set the run configuration's command to `run --release -- new --theme kitchen --reveal`, all lowercase.

## 8. Run the tests

```
cargo test --release
```

There are 7 tests. The slowest takes a few seconds, because it builds every built-in theme twice.

| test | what it checks |
|---|---|
| `lattice_has_eight_full_entries` | the basic grid shape has 4 across and 4 down entries |
| `templates_are_symmetric_and_valid` | 200 random grid shapes are all mirror-symmetric and legal |
| `b64_round_trip` | the share-code encoding decodes back to the same bytes |
| `enumeration_survives_code` | multi-word answers like `BIG CAT (3,3)` survive a share code |
| `windows_line_endings_and_bom` | theme files saved by Notepad or PowerShell (Windows line endings, a UTF-8 byte-order mark) load correctly |
| `custom_theme_survives_its_share_code` | a custom word list rebuilds from its code alone |
| `same_code_same_puzzle` | every built-in theme gives an identical puzzle from the same code |

Use `--release` here. The tests build real puzzles, and an unoptimized build is much slower.

## 9. Format and lint

```
cargo fmt
cargo clippy --all-targets -- -D warnings
```

`cargo fmt` rewrites the code into standard Rust formatting. `cargo clippy` is Rust's official code reviewer. The `-D warnings` flag makes it fail on any suggestion at all, so if this passes, the code is clean. It should pass with no output beyond `Finished`.

## 10. Commit and push to GitHub

First, set up `.gitignore` properly. `cargo new` only adds `/target`, but your editor also creates settings files that don't belong on GitHub. RustRover makes an `.idea/` folder and a `.iml` file, and VS Code makes `.vscode/`. Replace `.gitignore` with:

```
# Build output
/target

# Editor and IDE settings (RustRover/IntelliJ, VS Code, Visual Studio)
.idea/
*.iml
.vscode/
.vs/

# Operating system clutter
.DS_Store
Thumbs.db
desktop.ini

# Puzzles you generate while trying it out in the project folder.
# (The examples in samples/ are kept.)
/*.pdf
/*.svg

# Python leftovers from tools/build_dict.py
__pycache__/
```

Do this **before** your first commit. `.gitignore` only stops Git from picking up new files. Anything already committed stays tracked until you remove it (see the troubleshooting table).

Keep `Cargo.lock`. It belongs in the repo for a program like this.

```
git add .
git commit -m "Initial commit"
```

On GitHub, create a new repository named `ximenes`. Leave it completely empty: no README, no license, no .gitignore, since you already have all three and GitHub's versions would conflict. Then:

```
git remote add origin https://github.com/saedarm/ximenes.git
git branch -M main
git push -u origin main
```

The `repository` line in `Cargo.toml` already points at this address.

## 11. Install it on your machine

```
cargo install --path .
```

This builds a release version and copies it into Cargo's bin folder (`%USERPROFILE%\.cargo\bin` on Windows), which rustup already put on your PATH. Open a new terminal and you can drop the `cargo run --release --` part:

```
ximenes new --theme weather --reveal
```

Run it again after any code change to update your installed copy.

## 12. Publish to crates.io (optional)

Publishing lets anyone install Ximenes with `cargo install ximenes`. Read this first: **a published version is permanent.** It can't be deleted or overwritten, only "yanked" (hidden from new installs), and crate names are first come, first served.

1. Sign in at [crates.io](https://crates.io) with your GitHub account.
2. Verify your email address on the [account settings](https://crates.io/settings/profile) page. You can't publish without this.
3. Create an API token at [crates.io/settings/tokens](https://crates.io/settings/tokens). Copy it right away, because it's only shown once.
4. Give the token to Cargo:

   ```
   cargo login
   ```

   Paste the token when it asks.
5. Check what will be uploaded:

   ```
   cargo package --list
   ```

   You should see about 25 files: the source, the data, the themes, the samples' `.txt` files, the licenses and the README. No PDFs, and nothing from `target/`.
6. Do a full rehearsal. This builds the package exactly as crates.io will and stops before uploading:

   ```
   cargo publish --dry-run
   ```

   It should finish with `warning: aborting upload due to dry run`. That warning is the success message. The package is about 290 KB compressed, far under the 10 MB limit.
7. Commit everything first. `cargo publish` refuses to run with uncommitted changes.
8. Publish:

   ```
   cargo publish
   ```

For a later release, bump `version` in `Cargo.toml` (0.1.0 → 0.1.1 for fixes, 0.2.0 for new features), commit, and publish again.

## 13. Rebuilding the dictionary (optional)

You only need this if you want to change the word list. It needs Python:

```
pip install nltk wordfreq
python -c "import nltk; nltk.download('wordnet')"
python tools/build_dict.py > data/words.tsv
```

In older Windows PowerShell (5.1), `>` saves the file as UTF-16, and the program expects UTF-8. Use this instead:

```
python tools/build_dict.py | Out-File -Encoding utf8 data/words.tsv
```

A new dictionary changes which puzzle every existing share code produces. If you publish one, raise `CODE_VERSION` in `src/theme.rs` (from `"1"` to `"2"`). Old codes will then give a clear "this code is from format v1" error instead of quietly producing a different puzzle.

## When something goes wrong

| error | cause | fix |
|---|---|---|
| `package ID specification 'Ximenes' did not match any packages` | the package was asked for with a capital letter | use lowercase `ximenes` in commands and run configurations |
| `linker 'link.exe' not found` | the Visual Studio C++ Build Tools aren't installed | run the rustup installer again, or install "Desktop development with C++" from the Visual Studio Installer |
| `couldn't read ...\data\words.tsv` | a data or theme file is missing or in the wrong folder | check the paths in step 4 |
| `` `let` expressions in this position are unstable `` | Rust is older than 1.88 | `rustup update` |
| ``file not found for module `grid` `` (or another name) | there's a `mod` line for a file that isn't there yet | add the file, or remove the `mod` line until you do |
| a long share code doesn't open | it got cut or split when copying | copy the whole thing, and put it in quotes: `ximenes open "1.~RY5B..."` |
| tests take minutes | they ran without `--release` | `cargo test --release` |
| `.idea` or `.iml` files still show up in Git after updating `.gitignore` | they were committed before the ignore rule existed | `git rm -r --cached .idea`, plus `git rm --cached` for any `.iml` file, then commit. This only untracks them; the files stay on your disk |
