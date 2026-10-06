mod clue;
mod dict;
mod grid;
mod render;
mod rng;
mod theme;

use clap::{Args, Parser, Subcommand};
use std::process::ExitCode;

/// Ximenes: builds a themed 7x7 cryptic crossword, then explains every clue.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Set a new puzzle
    New {
        /// Built-in theme name or path to a word list (`WORD = definition` per line)
        #[arg(short, long, default_value = "kitchen")]
        theme: String,
        /// Any string; the same theme + seed always gives the same puzzle
        #[arg(short, long)]
        seed: Option<String>,
        #[command(flatten)]
        out: Output,
    },
    /// Rebuild a puzzle from its share code
    Open {
        code: String,
        #[command(flatten)]
        out: Output,
    },
    /// List the built-in themes
    Themes,
}

#[derive(Args)]
struct Output {
    /// Also print the solution and how each clue works
    #[arg(short, long)]
    reveal: bool,
    /// Write a printable SVG (add --answers to include the solution)
    #[arg(long)]
    svg: Option<String>,
    /// Write a two-page PDF: puzzle, then solution with explanations
    #[arg(long)]
    pdf: Option<String>,
    /// Include the solution page in the SVG
    #[arg(long)]
    answers: bool,
}

fn random_seed() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut r = rng::Rng::from_str(&t.to_string());
    const ALPHA: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    (0..6)
        .map(|_| ALPHA[r.below(ALPHA.len())] as char)
        .collect()
}

fn build(theme: &theme::Theme, seed: &str) -> Result<render::Puzzle, String> {
    let dict = dict::Dict::load();
    let code = theme::share_code(theme, seed);
    let mut rng = rng::Rng::from_str(&theme::seed_string(theme, seed));
    let setter = clue::Setter { dict: &dict };

    // A theme word with no fair wordplay would end up as a bare definition.
    let (words, skipped): (Vec<_>, Vec<_>) = theme
        .entries
        .iter()
        .cloned()
        .partition(|t| setter.clueable(t));
    for t in &skipped {
        eprintln!(
            "note: leaving out {} (no fair wordplay from the word list)",
            t.answer
        );
    }
    let mut memo = std::collections::HashMap::new();
    let mut clueable = |w: &str| {
        *memo
            .entry(w.to_string())
            .or_insert_with(|| setter.clueable_word(w))
    };
    let fill = grid::fill(&dict, &words, &mut rng.fork("grid"), 400, &mut clueable)
        .ok_or("couldn't fill a grid with this seed; try another")?;

    let mut order: Vec<&grid::Entry> = fill.entries.iter().collect();
    order.sort_by_key(|e| (!e.slot.across, e.slot.number));
    let mut used = Vec::new();
    let mut clues = Vec::new();
    for e in order {
        let mut r = rng.fork(&format!("clue{}{}", e.slot.number, e.slot.across));
        let c = setter.clue(e, &words, &used, &mut r);
        used.push(c.device);
        clues.push(c);
    }
    Ok(render::Puzzle {
        title: theme.title.clone(),
        code,
        fill,
        clues,
    })
}

fn emit(p: &render::Puzzle, out: &Output) -> Result<(), String> {
    print!("{}", render::text(p, out.reveal));
    let themed = p.fill.theme_count();
    eprintln!(
        "\n{themed} theme word{} in the grid",
        if themed == 1 { "" } else { "s" }
    );
    if let Some(path) = &out.svg {
        std::fs::write(path, render::svg(p, out.answers)).map_err(|e| format!("{path}: {e}"))?;
        eprintln!("wrote {path}");
    }
    if let Some(path) = &out.pdf {
        std::fs::write(path, render::pdf(p)).map_err(|e| format!("{path}: {e}"))?;
        eprintln!("wrote {path}");
    }
    Ok(())
}

fn run() -> Result<(), String> {
    match Cli::parse().cmd {
        Cmd::Themes => {
            for (name, title) in theme::builtin_names() {
                println!("{name:<18} {title}");
            }
            Ok(())
        }
        Cmd::New { theme, seed, out } => {
            let t = theme::load(&theme)?;
            let seed = seed.unwrap_or_else(random_seed);
            emit(&build(&t, &seed)?, &out)
        }
        Cmd::Open { code, out } => {
            let (t, seed) = theme::decode(&code)?;
            emit(&build(&t, &seed)?, &out)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ximenes: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_code_same_puzzle() {
        for (name, _) in theme::builtin_names() {
            let t = theme::load(name).unwrap();
            let a = render::text(&build(&t, "seed-42").unwrap(), true);
            let (t2, seed) = theme::decode(&format!("1.{name}.seed-42")).unwrap();
            let b = render::text(&build(&t2, &seed).unwrap(), true);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn custom_theme_survives_its_share_code() {
        let path = std::env::temp_dir().join("ximenes-test-theme.txt");
        std::fs::write(
            &path,
            "# Test\n\n  dice = they're rolled\nTOKEN = game piece\nCHESS = game of kings\n\
             POKER = card game with bluffing\nRULES = what the box insert explains\n",
        )
        .unwrap();
        let t = theme::load(path.to_str().unwrap()).unwrap();
        let a = build(&t, "friday").unwrap();
        let (t2, seed) = theme::decode(&a.code).unwrap();
        let b = build(&t2, &seed).unwrap();
        assert_eq!(render::text(&a, true), render::text(&b, true));
        // the puzzle is seeded from the word list, not the compressed bytes
        assert_eq!(t.seed_key, t2.seed_key);
    }
}
