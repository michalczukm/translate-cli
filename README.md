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

The config directory is `0700` and the file `0600`, the same posture as
`~/.aws/credentials`. macOS Keychain would encrypt it at rest, but its ACL binds to the
binary — every reinstall re-prompts for access, and it is unavailable over SSH without an
unlocked login keychain. The prompt only appears on a terminal; pipes and scripts still fail
with exit code 3.

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

Colour and the decorated block turn themselves off when the output is piped.

### Replacing `ten` / `tpl`

```zsh
tpl() { translate en:pl "$@" }
ten() { translate pl:en "$@" }
```

## Single words: word senses

A one-word source translated **into English** gets a `trans`-style dictionary block instead of
the alternative line — the synonyms come from [Datamuse](https://www.datamuse.com/api/)
(free, no key), filtered to the part of speech of the translated word:

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
    of a sudden

nagle
    suddenly
```

DeepL itself has no dictionary data — `/v2/translate` returns one string. Datamuse is
English-only, so single words translated *into* Polish keep the normal alternative line;
there is no free Polish thesaurus to match it.

## The alternative line

Always one extra API call, picked from the target language:

1. Target supported by DeepL Write (`DE, EN-GB, EN-US, ES, FR, IT, PT-BR, PT-PT`) → `/v2/write/rephrase`.
2. Otherwise a formality-capable target (`DE, ES, FR, IT, JA, NL, PL, PT-*, RU`) → a second
   translation with `formality=prefer_more`.
3. Neither → no alternative line. A failed lookup never fails the run.

Tier 2 exists because DeepL Write does not support Polish.

## Typos

Typos derail DeepL's *language detection*, not its translation. With an explicit pair the
translation already survives them:

```sh
translate pl:en "dzien dobri, co slychac?"   # -> Good morning, how are you?
translate :pl   "gud mroning, how ar yu?"    # -> detected Danish, garbage out
```

So always give both sides — `ten` and `tpl` do.

On top of that, when the source language is stated *and* supported by DeepL Write
(`DE, EN-GB, EN-US, ES, FR, IT, PT-*`), the source is spell-checked before translating:

```
$ tpl "gud mroning, how ar yu?"
gud mroning, how ar yu?
Did you mean: Good morning! How are you?

Dzień dobry! Jak się masz?
...
```

The translation comes from the corrected text. Nothing is printed when the text is already
clean, and Polish sources skip the check entirely — DeepL has no Write support for Polish and
handles Polish typos well on its own.

Calls per run: 2 normally, 3 when a correction pass applies.

## Why Rust

|  | Node + TypeScript + SEA | Rust |
|---|---|---|
| binary size | ~110 MB (bundles the whole node runtime) | ~2–4 MB static |
| cold start | ~45–60 ms | ~2–5 ms |
| build chain | esbuild → sea-config → blob → copy node → `codesign --remove-signature` → postject → re-sign | `cargo build --release` |
| install | hand-rolled symlink step | `cargo install --path .` |
| runtime deps | none (bundled) | none (static) |
| HTTP | built-in `fetch` | `ureq` |
| test mocking | stub global fetch | `httpmock` |

The tool is invoked dozens of times a day interactively, so node's startup cost is the one you
feel. The two API calls are strictly sequential — the alternative is derived from the
translation — so no async runtime is needed either.

`std::io::IsTerminal` and a raw ANSI escape cover the single bold span, so no colour crate.

## Development

```sh
just check      # clippy -D warnings, rustfmt, cargo test — no live API
just smoke      # hits the real API with the key from .env
```
