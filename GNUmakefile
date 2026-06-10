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
	docker pull mcr.microsoft.com/devcontainers/typescript-node
.PHONY: setup

down: must-run-outside
	docker rm -f tindalwic-devcontainer-vscode
	docker image rm $$(docker image ls -q --filter "reference=vsc-tindalwic*")
.PHONY: down

httpd: must-run-outside
	cd target
	python -m http.server >&http.server.log
.PHONY: httpd

# =====================================================================================

all: fmt doc main/test main/api main/llvm-lines webapp msrv main/rand
.PHONY: all

test: main/test main/rand
.PHONY: test

main/test: must-run-inside
	cd main
	set -e
	echo ====== default ; cargo test
	echo ====== alloc ; cargo test --features alloc
	echo ====== bumpalo ; cargo test --features bumpalo
	echo ====== serde ; cargo test --features serde
	echo ====== all ; cargo test --all-features
.PHONY: main/test

main/rand: must-run-inside
	cd main
	cargo bench --all-features --bench rand
.PHONY: main/rand

doc: must-run-inside
	cargo doc --all-features --no-deps # --document-private-items
.PHONY: doc

BINSTALL = binstall --no-confirm --only-signed --disable-telemetry

webapp: must-run-inside
	set -e
	cd webapp
	WASM=wasm32-unknown-unknown
	rustup target list --installed | grep -q $$WASM || rustup target add $$WASM
	cargo build --target $$WASM --profile dev
	cargo build --target $$WASM --profile release-small
	VER=$$(cargo pkgid -p wasm-bindgen | sed -E -e 's=^[^@]+@([0-9.]+).*$$=\1=')
	cargo install --list | grep -q "wasm-bindgen-cli v$$VER:" \
	    || cargo $(BINSTALL) wasm-bindgen-cli --version $$VER
	cd ../target
	rm -rf webapp-*
	NAME=tindalwic_webapp
	wasm-bindgen --target web --keep-debug \
	    --out-dir webapp-dev $$WASM/debug/$$NAME.wasm
	wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section \
	    --out-dir webapp-release $$WASM/release-small/$$NAME.wasm
	cp ../webapp/favicon.ico ./
	cp ../webapp/{index.html,favicon.ico} webapp-dev/
	cp ../webapp/{index.html,favicon.ico} webapp-release/
	cargo install --list | grep -q wasm-opt || cargo $(BINSTALL) wasm-opt
	cd webapp-release
	wasm-opt -Oz --enable-bulk-memory -o $${NAME}_bg.wasm $${NAME}_bg.wasm
.PHONY: webapp

nightly: must-run-inside
	rustup toolchain list | grep -q nightly || rustup toolchain install nightly
.PHONY: nightly

main/api: nightly
	set -e
	cargo install --list | grep -q cargo-public-api || cargo $(BINSTALL) cargo-public-api
	mkdir -p target/public-api/{all,default}
	cd main
	cargo public-api --target-dir ../target/public-api/default \
	  >../target/public-api/tindalwic-default.api
	cargo public-api --all-features --target-dir ../target/public-api/all \
	  >../target/public-api/tindalwic-all.api
	cat ../target/public-api/tindalwic-all.api \
	  | sed -E -e 's=^impl (.*)=|\1|impl|=' \
	  | sed -E -e 's=^(impl<[^>]+>) (.*)=|\2|\1|=' \
	  | sed -E -e 's=^pub (enum|fn|const fn|mod|struct|use|type) (&?)(.*)=|\3|\2\1|=' \
	  | sed -E -e 's=^pub (.*)=|\1|property|=' \
	  | LC_ALL=C sort -u >../target/public-api/tindalwic-all.org
.PHONY: main/api

main/llvm-lines: must-run-inside
	cd main
	cargo install --list | grep -q cargo-llvm-lines || cargo $(BINSTALL) cargo-llvm-lines
	cargo llvm-lines --all-features >../target/llvm-lines.out
.PHONY: main/llvm-lines

fmt: nightly
	: see comment near top of main/src/serde/mod.rs
	sed -i \
	  -e 's|^seeded! {$$|const _: () = {|' \
	  -e 's|^} // !seeded$$|}; // !seeded|' \
	  main/src/serde/*.rs
	rustfmt +nightly $$(find macros main webapp -name '*.rs')
	sed -i \
	  -e 's|^const _: () = {$$|seeded! {|' \
	  -e 's|^}; // !seeded$$|} // !seeded|' \
	  main/src/serde/*.rs
.PHONY: fmt

msrv: must-run-inside
	set -e
	cargo install --list | grep -q cargo-msrv || cargo $(BINSTALL) cargo-msrv
	echo ====== macros ; cargo msrv verify --path macros
	echo ====== main   ; cargo msrv verify --path main
	echo ====== webapp ; cargo msrv verify --path webapp
.PHONY: msrv

ghraw: must-run-outside ;@
	PROJECT='https://raw.githubusercontent.com/comments-are-important/tindalwic'
	COMMIT="$$(git rev-parse HEAD)"
	if git merge-base --is-ancestor "$$COMMIT" origin/main
	then
	  git log -1
	  git ls-tree -r --format "$$PROJECT/$$COMMIT/%(path)" HEAD
	else
	  echo "HEAD is not known to be in origin/main... push (maybe fetch too)"
	fi
.PHONY: ghraw

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
