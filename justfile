bin_dir := env('HOME') / ".local/bin"
binary := justfile_directory() / "target/release/translate"

# list the recipes
default:
    @just --list

# release build
build:
    cargo build --release

# symlink the release build into ~/.local/bin (rebuilds go live immediately)
link: build
    @mkdir -p {{bin_dir}}
    @ln -sf {{binary}} {{bin_dir}}/translate
    @echo "linked {{bin_dir}}/translate -> {{binary}}"
    @command -v translate

# remove the symlink
unlink:
    @rm -f {{bin_dir}}/translate
    @echo "removed {{bin_dir}}/translate"

# unit and integration tests, no live API
test:
    cargo test

# clippy and rustfmt, both as errors
lint:
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check

# hit the real DeepL API with the key from .env
smoke: build
    ./scripts/smoke.sh

# what a commit should pass
check: lint test

# tag the Cargo.toml version, push the tag and point the Homebrew formula at it
brew-publish: check
    #!/usr/bin/env bash
    set -euo pipefail
    git diff --quiet HEAD || { echo "working tree not clean" >&2; exit 1; }
    version=$(cargo pkgid | sed 's/.*[#@]//')
    git tag "v$version"
    git push origin "v$version"
    url="https://github.com/michalczukm/translate-cli/archive/refs/tags/v$version.tar.gz"
    sha=$(curl -fsSL "$url" | shasum -a 256 | cut -d' ' -f1)
    tap=$(mktemp -d)
    trap 'rm -rf "$tap"' EXIT
    gh repo clone michalczukm/homebrew-tap "$tap" -- -q
    perl -pi -e "s|^  url .*|  url \"$url\"|; s|^  sha256 .*|  sha256 \"$sha\"|" "$tap/Formula/translate-cli.rb"
    git -C "$tap" commit -qam "translate-cli $version"
    git -C "$tap" push -q
    echo "published translate-cli $version: brew upgrade translate-cli"
