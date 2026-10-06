"""Builds data/words.tsv from WordNet + wordfreq.

Run once; the output is committed and baked into the binary, so the Rust
side never needs Python. Output lines:

    WORD <tab> zipf*10 <tab> senses

senses is empty for "plain" words (usable as anagram fodder / hidden-word
phrases but not as clue parts), otherwise `lex=def|def;lex=def` where lex is
the WordNet lexname (so double definitions can insist on two different senses).

    pip install nltk wordfreq
    NLTK_ALLOW_PROXIED_URLOPEN=1 python -c "import nltk; nltk.download('wordnet')"
    python tools/build_dict.py > data/words.tsv
"""
import re
import sys
from nltk.corpus import wordnet as wn
from wordfreq import zipf_frequency, top_n_list

ALPHA = re.compile(r"^[a-z]+$")
ROMAN = re.compile(r"^m{0,3}(cm|cd|d?c{0,3})(xc|xl|l?x{0,3})(ix|iv|v?i{0,3})$")
BAD_DOMAINS = {"vulgarism", "obscenity", "ethnic_slur", "disparagement", "slang", "derogation"}
BLOCK = set("""
nazi rape rapist slut whore bitch bastard cunt fuck shit piss crap dick cock prick twat tits
penis vagina anus anal sex sexy porn nigger negro spic kike chink gook dyke fag faggot queer
retard homo gay ki jap wop coon paki gypsy tranny kill suicide
""".split())
# Words that may appear in a surface even though WordNet has no entry for them.
FUNCTION = set("""
a an and or but if of to in on at by for with from into onto upon is are was were be been am has
had have do does did not no so as it its he she we they you your our my his her their them us me
him this that these those who whom what which when where why how all any some each every few many
much more most other such than then there here very too also just only own same both either
neither nor yet off out over under again once about above below after before since until while
because though although whether shall will would should could can may might must let
""".split())
# single-letter and 2-letter entries come from abbrev.tsv instead
MIN_LEN, MAX_LEN = 3, 7
# Hypernyms too vague to be a fair definition of anything.
GENERIC = {
    "entity", "object", "thing", "physical object", "natural object", "abstract entity",
    "abstraction", "person", "someone", "somebody", "individual", "causal agent", "organism",
    "being", "act", "event", "group", "state", "attribute", "relation", "measure", "matter",
    "whole", "unit", "part", "artifact", "artefact", "activity", "action", "process",
    "communication", "cognition", "psychological feature", "content", "instrumentality",
}


def is_numeral(w):
    """Roman numerals (iii, xii) read as junk in a grid; 'mix' is still a word."""
    if len(w) < 2 or not ROMAN.match(w):
        return False
    syns = wn.synsets(w)
    return not syns or any(k in syns[0].definition() for k in ("number", "numeral", "cardinal"))


def zipf(w):
    return zipf_frequency(w, "en")


def stem_clash(word, other):
    o = other.replace(" ", "").replace("-", "")
    return word in o or o in word or o[:4] == word[:4]


def sense_defs(word, syn):
    cands = []
    for l in syn.lemmas():
        n = l.name().replace("_", " ")
        if n.lower() != n or n == word or not re.fullmatch(r"[a-z ]+", n):
            continue
        if stem_clash(word, n) or len(n.split()) > 2:
            continue
        f = min(zipf(p) for p in n.split())
        if f >= 3.5 and not set(n.split()) & BLOCK:
            cands.append((f + (0.4 if " " not in n else 0), n))
    if not cands and syn.pos() in "nv":
        for h in syn.hypernyms()[:2]:
            for l in h.lemmas()[:3]:
                n = l.name().replace("_", " ")
                if n.lower() == n and re.fullmatch(r"[a-z ]+", n) and not stem_clash(word, n) and len(n.split()) <= 2:
                    f = min(zipf(p) for p in n.split())
                    if f >= 3.5 and n not in GENERIC:
                        cands.append((f - 10.0, "^" + n))  # hypernym: looser, never first
    if not cands:
        g = syn.definition()
        g = re.sub(r"\(.*?\)", "", g).strip().split(";")[0].strip()
        if 1 <= len(g.split()) <= 4 and word not in g and re.fullmatch(r"[a-z ,'-]+", g):
            cands.append((-20.0, "@" + g))
    cands.sort(key=lambda c: -c[0])
    seen, out = set(), []
    for _, n in cands:
        if n not in seen:
            seen.add(n)
            out.append(n)
    return out[:3]

# Output markers: plain = synonym, ^ = hypernym (looser), @ = short gloss.


def main():
    words = {}
    for w in top_n_list("en", 60000):
        if ALPHA.match(w) and 2 <= len(w) <= 9 and w not in BLOCK and not is_numeral(w) and re.search("[aeiouy]", w):
            words[w] = zipf(w)

    rows = []
    for w, f in words.items():
        senses = []
        if MIN_LEN <= len(w) <= MAX_LEN and f >= 2.6:
            lemma_syns = [s for s in wn.synsets(w) if any(l.name() == w for l in s.lemmas())]
            bad = False
            for s in lemma_syns[:6]:
                doms = {d.name().split(".")[0] for d in s.usage_domains()}
                if doms & BAD_DOMAINS or "offensive" in s.definition() or "slur" in s.definition():
                    bad = True
                    break
                defs = sense_defs(w, s)
                if defs:
                    senses.append((s.lexname(), defs))
            if bad:
                continue
        # plain words: frequent enough to read naturally in a hidden/anagram phrase,
        # and a real lowercase word (drops names like Canada and junk like "im")
        if not senses:
            if f < 3.4:
                continue
            real = any(l.name() == w for s in wn.synsets(w) for l in s.lemmas()) or (
                len(w) > 2 and wn.morphy(w) is not None and any(
                    l.name() == wn.morphy(w) for s in wn.synsets(w) for l in s.lemmas()))
            if w not in FUNCTION and (not real or len(w) == 2):
                continue
        sense_txt = ";".join(f"{lex}={'|'.join(d)}" for lex, d in senses[:4])
        rows.append((w.upper(), int(round(f * 10)), sense_txt))

    rows.sort()
    for w, f, s in rows:
        print(f"{w}\t{f}\t{s}")
    print(f"{len(rows)} rows, {sum(1 for r in rows if r[2])} defined", file=sys.stderr)


if __name__ == "__main__":
    main()
