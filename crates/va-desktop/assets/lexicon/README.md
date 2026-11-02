# Prompt spellcheck lexicon

| File | Source | License |
| --- | --- | --- |
| `../dictionary.txt` | [dwyl/english-words](https://github.com/dwyl/english-words) `words_alpha.txt` | Public domain (Unlicense) |
| `../bands.txt` | [SCOWL 2020.12.07](http://wordlist.aspell.net/), `english-words.*` plus the US/UK/CA/AU variant deltas | Permissive, see `LICENSE-SCOWL.txt` |
| `../vocab.txt` | Hand-maintained project and software vocabulary | Same as the project (Apache-2.0) |

`dictionary.txt` is the broad membership list (sorted, lowercase, `a-z` only),
`bands.txt` maps common words to a SCOWL commonness band (`10` most common
through `70`), and `vocab.txt` keeps technical terms from being flagged.
`spell.rs` embeds the three files and binary-searches them.
