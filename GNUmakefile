.ONESHELL:
SHELL = bash

usage: ;@
	if which -s less grep
	then
	  grep --color=always -E '|^#.*|`[^`]*`' README.md | less -R --use-color
	elif which -s pager
	then
	  pager README.md
	else
	  more README.md
	fi
.PHONY: usage

must-run-outside: ;@
	if [[ -n "$$TINDALWIC_CID" && "$$(< /tmp/.devcontainerId)" == "$$TINDALWIC_CID" ]]
	then
	  echo 'must run outside devcontainer'
	  exit 1
	fi
.PHONY: must-run-outside

must-run-inside: ;@
	if [[ -z "$$TINDALWIC_CID" || "$$(< /tmp/.devcontainerId)" != "$$TINDALWIC_CID" ]]
	then
	  echo 'must run inside devcontainer'
	  exit 1
	fi
	set -ex
.PHONY: must-run-inside

# =====================================================================================

setup: must-run-outside ;@
	code --install-extension ms-vscode-remote.remote-containers \
	  | sed -e 's= is already installed[.].*= is already installed.='
.PHONY: setup

down: must-run-outside
	docker rm -f tindalwic-devcontainer-vscode
.PHONY: down

# =====================================================================================

python/test: must-run-inside
	uv run -- python -m tindalwic_test
.PHONY: python/test

python/repl: must-run-inside
	uv run -- python -i -c "import tindalwic_test;from tindalwic import *"
.PHONY: python/repl

python/profile: must-run-inside
	rm -f /tmp/tindalwic.pstats
	set -e
	uv run -- python -m tindalwic_test --pstats=/tmp/tindalwic.pstats --loops=10000
	uv run -- snakeviz /tmp/tindalwic.pstats
.PHONY: python/profile

python/coverage: must-run-inside
	mkdir -p /tmp/tindalwic.coverage
	cd /tmp/tindalwic.coverage
	set -e
	uv run -- coverage run --branch --source=tindalwic -m tindalwic_test
	uv run -- coverage report --fail-under=100 && exit
	uv run -- coverage html --directory=.
	uv run -- python -m http.server
.PHONY: python/coverage

python/build: python/test
	uv build --sdist
.PHONY: python/build

# =====================================================================================

rust/test: must-run-inside
	cd rust
	cargo test -q
	cargo test -q --features alloc
.PHONY: rust/test

rust/doc: must-run-inside
	cd rust
	cargo doc --all-features --no-deps # --document-private-items
	cd target/doc
	uv run -- python -m http.server >&http.server.log
.PHONY: rust/doc

rust/nightly: must-run-inside
	cd rust
	rustup toolchain list | grep -q nightly || rustup toolchain install nightly
.PHONY: rust/nightly

rust/api: rust/nightly
	cd rust
	cargo install --list | grep -q cargo-public-api || cargo install cargo-public-api
	mkdir -p target
	cargo public-api -sss --all-features --target-dir target/public-api \
	  | grep -v '^impl' \
	  | sed -E -e 's=^pub (enum|fn|const fn|mod|struct|use|type) (.*)=|\2|\1|=' \
	  | sed -E -e 's=^pub (.*)=|\1|property|=' \
	  | LC_ALL=C sort >target/public-api/tindalwic.org
.PHONY: rust/api

rust/llvm-lines: must-run-inside
	cd rust
	cargo install --list | grep -q cargo-llvm-lines || cargo install cargo-llvm-lines
	cargo llvm-lines --all-features
.PHONY: rust/llvm-lines

rust/fmt: rust/nightly
	cd rust
	rustfmt +nightly --config format_code_in_doc_comments=true src/lib.rs
.PHONY: rust/fmt

# =====================================================================================
