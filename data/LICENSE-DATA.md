# Licenses for the word data

The Ximenes source code is MIT licensed (see `LICENSE` in the project root).
The files in this folder are data, built by `tools/build_dict.py`, and carry
their own terms because they are derived from other people's work.

## data/words.tsv

Built from two sources:

**WordNet 3.0**, which supplies the definitions and sense categories.
WordNet's license requires this notice to appear on all copies:

> WordNet 3.0 Copyright 2006 by Princeton University. All rights reserved.
>
> THIS SOFTWARE AND DATABASE IS PROVIDED "AS IS" AND PRINCETON UNIVERSITY
> MAKES NO REPRESENTATIONS OR WARRANTIES, EXPRESS OR IMPLIED. BY WAY OF
> EXAMPLE, BUT NOT LIMITATION, PRINCETON UNIVERSITY MAKES NO REPRESENTATIONS
> OR WARRANTIES OF MERCHANTABILITY OR FITNESS FOR ANY PARTICULAR PURPOSE OR
> THAT THE USE OF THE LICENSED SOFTWARE, DATABASE OR DOCUMENTATION WILL NOT
> INFRINGE ANY THIRD PARTY PATENTS, COPYRIGHTS, TRADEMARKS OR OTHER RIGHTS.

The full WordNet license: https://wordnet.princeton.edu/license-and-commercial-use

**wordfreq** by Robyn Speer, which supplies the word frequencies and the
choice of which words are common enough to include. wordfreq's data is
distributed under the Creative Commons Attribution-ShareAlike 4.0 license
(https://creativecommons.org/licenses/by-sa/4.0/). Source:
https://github.com/rspeer/wordfreq

Because `words.tsv` is derived from that data, `words.tsv` is also released
under CC BY-SA 4.0. You may share and adapt it, as long as you give credit
and release your changes under the same license.

## data/abbrev.tsv

Standard cryptic-crossword abbreviations, compiled by hand for this project.
MIT licensed along with the code.
