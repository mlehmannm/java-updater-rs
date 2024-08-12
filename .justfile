# justfile (https://github.com/casey/just)

# load a .env file, if present
set dotenv-load

# set shell for non-Windows OSs / dev container
set shell := ["pwsh", "-NoLogo", "-Command"]

# set shell for Windows OSs
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

# select toolchain [e.g.: just CHANNEL=<channel> recipe args]
CHANNEL := ''
#CHANNEL := '+stable'
#CHANNEL := '+nightly'
#CHANNEL := '+1.54'

# configure log
export RUST_LOG := 'trace,cargo=warn,hyper=warn,reqwest=warn'

# capture backtraces
export RUST_BACKTRACE := '1'

# default
[private]
default:
    @{{ just_executable() }} --list --unsorted --justfile "{{ justfile() }}"

# build
build *ARGS='':
    cargo {{ CHANNEL }} build {{ ARGS }}

# build in release mode
build-release *ARGS='':
    cargo {{ CHANNEL }} build --release {{ ARGS }}

# run (specific) tests
test *ARGS='':
    cargo {{ CHANNEL }} test --all-features --all-targets {{ ARGS }}

# run (specific) tests without capturing its output
test-nocapture *ARGS='':
    cargo {{ CHANNEL }} test --all-features --all-targets -- --nocapture {{ ARGS }}

# run (specific) tests in release mode
test-release *ARGS='':
    cargo {{ CHANNEL }} test --release --all-features --all-targets {{ ARGS }}

# run
run *ARGS='':
    cargo {{ CHANNEL }} run -- {{ ARGS }}

# run (wih all features)
run-full *ARGS='':
    cargo {{ CHANNEL }} run --all-features -- {{ ARGS }}

# install
install *ARGS='': (test "--release")
    cargo {{ CHANNEL }} install --locked --path . {{ ARGS }}

# install (debug)
install-debug *ARGS='':
    cargo {{ CHANNEL }} install --debug --locked --path . {{ ARGS }}

# clean
clean *ARGS='':
    cargo {{ CHANNEL }} clean {{ ARGS }}

# check
check *ARGS='':
    cargo {{ CHANNEL }} check --all-features --all-targets {{ ARGS }}

# clippy
clippy *ARGS='':
    cargo {{ CHANNEL }} clippy --all-features --all-targets --no-deps --tests {{ ARGS }}

# clippy in release mode (warnings will be treated as errors)
clippy-release *ARGS='':
    cargo {{ CHANNEL }} clippy --all-features --all-targets --tests -- -D warnings {{ ARGS }}

# generate documentation
doc *ARGS='':
    cargo {{ CHANNEL }} doc --all-features --no-deps --open {{ ARGS }}

# format
fmt *ARGS='':
    cargo {{ CHANNEL }} fmt --all {{ ARGS }}

# release a new version
release version='--help':
    cargo {{ CHANNEL }} release {{ version }} --execute

# create the icon from svg via imagemagick (docker)
ico:
    docker run --rm -v {{ invocation_directory() }}:/app -w /app minidocks/imagemagick convert -density 256x256 -background transparent res/svg/exe.svg -define icon:auto-resize=256,64,48,40,32,24,20,16 -compress none res/exe.ico

# check for outdated dependencies / upgrade dependencies / update dependencies
update-deps:
    cargo outdated --verbose
    cargo upgrade --recursive true --verbose
    cargo update --recursive --verbose

# install prerequisites (unix)
[unix]
install-prereqs: install-prereqs-cargo

# install prerequisites (windows)
[windows]
install-prereqs: install-prereqs-cargo
    -winget update --id nektos.act

# install prerequisites
[private]
install-prereqs-cargo:
    cargo install --locked cargo-cache cargo-edit cargo-expand cargo-outdated cargo-release
