//! Themes (word lists with definitions) and share codes.
//!
//! A share code is `1.<theme>.<seed>`. For built-in themes <theme> is the name;
//! for a custom list it is `~` + the list itself, deflated and base64url'd, so
//! the code alone is enough to rebuild the puzzle on someone else's machine.

const BUILTIN: &[(&str, &str)] = &[
    ("market-garden", include_str!("../themes/market-garden.txt")),
    ("cosmic-encounter", include_str!("../themes/cosmic-encounter.txt")),
    ("dayton", include_str!("../themes/dayton.txt")),
];

pub const CODE_VERSION: &str = "1";

#[derive(Clone)]
pub struct ThemeEntry {
    /// Letters only, uppercase: what goes in the grid.
    pub answer: String,
    /// "(5,7)" style enumeration.
    pub enumeration: String,
    pub def: String,
}

pub struct Theme {
    pub title: String,
    pub entries: Vec<ThemeEntry>,
    /// Either a built-in name or `~payload`.
    pub code_part: String,
}

pub fn builtin_names() -> Vec<(&'static str, String)> {
    BUILTIN.iter().map(|(n, t)| (*n, parse(t, "").map(|t| t.title).unwrap_or_default())).collect()
}

pub fn parse(text: &str, code_part: &str) -> Result<Theme, String> {
    let mut title = String::from("Untitled");
    let mut entries = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(t) = line.strip_prefix('#') {
            if entries.is_empty() && title == "Untitled" {
                title = t.trim().to_string();
            }
            continue;
        }
        let (word, def) = line
            .split_once('=')
            .or_else(|| line.split_once(':'))
            .ok_or_else(|| format!("line {}: expected `WORD = definition`", n + 1))?;
        let (word, def) = (word.trim(), def.trim());
        if def.is_empty() {
            return Err(format!("line {}: `{}` needs a definition", n + 1, word));
        }
        let groups: Vec<String> = word
            .split(|c: char| c == ' ' || c == '-')
            .map(|g| g.chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>())
            .filter(|g| !g.is_empty())
            .collect();
        let answer: String = groups.concat().to_ascii_uppercase();
        if answer.len() < 3 || answer.len() > 7 {
            eprintln!("note: skipping {word} (needs 3-7 letters to fit a 7x7 grid)");
            continue;
        }
        let sep = if word.contains('-') { "-" } else { "," };
        let enumeration = format!(
            "({})",
            groups.iter().map(|g| g.len().to_string()).collect::<Vec<_>>().join(sep)
        );
        entries.push(ThemeEntry { answer, enumeration, def: def.to_string() });
    }
    if entries.is_empty() {
        return Err("theme has no usable words (need 3-7 letters each)".into());
    }
    Ok(Theme { title, entries, code_part: code_part.to_string() })
}

/// `--theme` accepts a built-in name or a path to a text file.
pub fn load(name_or_path: &str) -> Result<Theme, String> {
    if let Some((n, t)) = BUILTIN.iter().find(|(n, _)| *n == name_or_path) {
        return parse(t, n);
    }
    let text = std::fs::read_to_string(name_or_path)
        .map_err(|e| format!("no built-in theme or readable file called `{name_or_path}`: {e}"))?;
    // Normalise before encoding so comments/whitespace don't bloat the code.
    let theme = parse(&text, "")?;
    let normal = normalise(&theme);
    let packed = miniz_oxide::deflate::compress_to_vec(normal.as_bytes(), 10);
    let code_part = format!("~{}", b64_encode(&packed));
    parse(&normal, &code_part)
}

fn normalise(t: &Theme) -> String {
    let mut s = format!("# {}\n", t.title);
    for e in &t.entries {
        // Keep word breaks so the enumeration survives the round trip.
        let mut word = String::new();
        let groups: Vec<usize> = e.enumeration[1..e.enumeration.len() - 1]
            .split(|c| c == ',' || c == '-')
            .filter_map(|g| g.parse().ok())
            .collect();
        let sep = if e.enumeration.contains('-') { "-" } else { " " };
        let mut i = 0;
        for (k, g) in groups.iter().enumerate() {
            if k > 0 {
                word.push_str(sep);
            }
            word.push_str(&e.answer[i..i + g]);
            i += g;
        }
        s.push_str(&format!("{} = {}\n", word, e.def));
    }
    s
}

pub fn share_code(theme: &Theme, seed: &str) -> String {
    format!("{CODE_VERSION}.{}.{seed}", theme.code_part)
}

/// Returns (theme, seed) from a share code.
pub fn decode(code: &str) -> Result<(Theme, String), String> {
    let mut it = code.trim().splitn(3, '.');
    let (Some(v), Some(t), Some(seed)) = (it.next(), it.next(), it.next()) else {
        return Err("share codes look like 1.<theme>.<seed>".into());
    };
    if v != CODE_VERSION {
        return Err(format!("code is from ximenes format v{v}; this build reads v{CODE_VERSION}"));
    }
    let theme = if let Some(payload) = t.strip_prefix('~') {
        let bytes = b64_decode(payload).ok_or("corrupt theme payload in code")?;
        let raw = miniz_oxide::inflate::decompress_to_vec(&bytes).map_err(|_| "corrupt theme payload in code")?;
        parse(&String::from_utf8_lossy(&raw), t)?
    } else {
        let (n, text) = BUILTIN.iter().find(|(n, _)| *n == t).ok_or_else(|| format!("unknown built-in theme `{t}` in code"))?;
        parse(text, n)?
    };
    Ok((theme, seed.to_string()))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn b64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, &b)| acc | (b as u32) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let vals: Vec<u32> = s.bytes().map(|c| B64.iter().position(|&b| b == c).map(|p| p as u32)).collect::<Option<_>>()?;
    let mut out = Vec::new();
    for chunk in vals.chunks(4) {
        if chunk.len() < 2 {
            return None;
        }
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, &v)| acc | v << (18 - 6 * i));
        for i in 0..chunk.len() - 1 {
            out.push((n >> (16 - 8 * i) & 255) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b64_round_trip() {
        for len in 0..20 {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(b64_decode(&b64_encode(&data)).unwrap(), data);
        }
    }

    #[test]
    fn enumeration_survives_code() {
        let t = parse("# T\nBIG CAT = feline\nRED-EYE = flight\nCAT = pet", "").unwrap();
        let normal = normalise(&t);
        let back = parse(&normal, "").unwrap();
        assert_eq!(back.entries[0].enumeration, "(3,3)");
        assert_eq!(back.entries[1].enumeration, "(3-3)");
        assert_eq!(back.entries[2].enumeration, "(3)");
    }
}
