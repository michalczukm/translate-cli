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

# copy the binary into ~/.cargo/bin instead of linking
install:
    cargo install --path .

# drop the installed copy
uninstall:
    cargo uninstall translate-cli

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

# store a DeepL API key
key: build
    @{{binary}} auth set-key
