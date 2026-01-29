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
	if [[ -n "$$ALACS_CID" && "$$(< /tmp/.devcontainerId)" == "ALACS=$$ALACS_CID" ]]
	then
	  echo 'must run outside devcontainer'
	  exit 1
	fi
.PHONY: must-run-outside

must-run-inside: ;@
	if [[ -z "$$ALACS_CID" || "$$(< /tmp/.devcontainerId)" != "ALACS=$$ALACS_CID" ]]
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
	docker rm -f ALACS-devcontainer-vscode
.PHONY: down

# =====================================================================================

python/test: must-run-inside
	uv run -- python -m alacs_test
.PHONY: python/test

python/repl: must-run-inside
	uv run -- python -i -c "import alacs_test;from alacs import *"
.PHONY: python/repl

python/profile: must-run-inside
	rm -f /tmp/ALACS.pstats
	set -e
	uv run -- python -m alacs_test --pstats=/tmp/ALACS.pstats --loops=10000
	uv run -- snakeviz /tmp/ALACS.pstats
.PHONY: python/profile

python/coverage: must-run-inside
	mkdir -p /tmp/ALACS.coverage
	cd /tmp/ALACS.coverage
	set -e
	uv run -- coverage run --branch --source=alacs -m alacs_test
	uv run -- coverage report --fail-under=100 && exit
	uv run -- coverage html --directory=.
	uv run -- python -m http.server
.PHONY: python/coverage

python/build: python/test
	uv build --sdist
.PHONY: python/build

# =====================================================================================
