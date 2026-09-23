# translate-cli

A tiny `translate` binary for the terminal. Give it a language pair and some text, get the
DeepL translation plus one alternative phrasing. A minimal reproduction of the `translate`
slice of [deepl-cli](https://github.com/DeepL/deepl-cli).

```
$ translate pl:en "dzień dobry, co słychać?"
dzień dobry, co słychać?

good morning, how are you?

Translations of dzień dobry, co słychać?
[ Polski -> English ]

    Good morning! How are you?
```

## Lazy shell functions for the two most common (for me) pairs

```zsh
tpl() { translate en:pl "$@" }
ten() { translate pl:en "$@" }
```

## Install

```sh
just link       # build + symlink into ~/.local/bin, rebuilds go live at once
just install    # or copy into ~/.cargo/bin via cargo install
```

`just` on its own lists every recipe. Without `just`: `cargo install --path .`.

## API key

First run without a key asks for one and stores it:

```
$ translate pl:en "dzień dobry"
No DeepL API key found. Get a free API key at https://www.deepl.com/pro-api
DeepL API key:
Saved to ~/.config/translate-cli/.env (0600)
```

Or set it up ahead of time:

```sh
translate auth set-key            # prompts, input hidden
translate auth set-key KEY        # non-interactive
translate auth path               # where it lives
```

The key is looked up in this order:

1. `DEEPL_API_KEY` environment variable
2. `~/.config/translate-cli/.env`
3. `./.env`

Free keys (ending in `:fx`) hit `api-free.deepl.com`, the rest `api.deepl.com`.

## Usage

```sh
translate pl:en "dzień dobry"        # explicit pair
translate :de "good morning"         # auto-detect the source
translate de "good morning"          # same thing, shorter
translate pl:en dzień dobry          # quotes optional
echo "good morning" | translate :pl  # stdin when no text argument
translate --plain pl:en "dzień dobry"  # translation + alternative, no decoration
translate usage                      # characters used this month
```

Colour turns itself off when the output is piped; `--plain` also drops the block.

## Single words

One word translated **into English** gets a `trans`-style dictionary block instead of the
alternative line, synonyms from [Datamuse](https://www.datamuse.com/api/) (free, no key):

```
$ ten nagle
nagle

suddenly

Definitions of nagle
[ Polski -> English ]

adverb
    suddenly
    dead
    short
    abruptly
    all of a sudden

nagle
    suddenly
```

DeepL has no dictionary data and Datamuse is English-only, so `en:pl` keeps the alternative
line. `--plain` skips the lookup.

## The alternative line

One extra call, chosen by target language: DeepL Write (`DE, EN-GB, EN-US, ES, FR, IT, PT-*`),
else a second translation with `formality=prefer_more` (`DE, ES, FR, IT, JA, NL, PL, PT-*, RU`),
else nothing. Tier 2 exists because Write has no Polish. A failed lookup never fails the run.

## Typos

Typos derail DeepL's *language detection*, not its translation — so always give both sides,
as `ten` and `tpl` do:

```sh
translate pl:en "dzien dobri, co slychac?"   # -> Good morning, how are you?
translate :pl   "gud mroning, how ar yu?"    # -> detected Danish, garbage out
```

Where DeepL Write covers the source language (not Polish), the source is spell-checked first
and the fix shown as `Did you mean: ...`; the translation then comes from the corrected text.

## Development

```sh
just check      # clippy -D warnings, rustfmt, cargo test — no live API
just smoke      # hits the real API with the key from .env
```
