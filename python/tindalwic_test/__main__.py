import sys
from pathlib import Path
from typing import Annotated

from rich.progress import track
from typer import Typer, Exit, Option

from . import TimedTindalwic, TimedRuamel, unit_tests
from .equals import diff_any, diff_translate, diff_ruamel
from .generate import Random


def FAILED(message: str):
    print(f"FAILED {message}", file=sys.stderr)
    raise Exit(code=1)


app = Typer()
profile_option = Option(
    help="write profile stats here (fail if already exists)",
    file_okay=False,
    dir_okay=False,
)
loops_option = Option(
    help="number of repetitions",
    min=0,
)
deepest_option = Option(
    help="limit the depth of generated random data structure",
    min=0,
)
widest_option = Option(
    help="limit the breadth of generated random data structure",
    min=0,
)


@app.command(
    help="""Run unit tests, then exercise the API with random data.

    Without the `pstats` option broad timing information is gathered and the ruamel.yaml
    conversions are included. The `pstats` option switches to detailed cProfile stats,
    focused only on the Tindalwic library (ruamel.yaml conversions are skipped).""",
)
def main(
    pstats: Annotated[Path | None, profile_option] = None,
    loops: Annotated[int, loops_option] = 250,
    deepest: Annotated[int, deepest_option] = 6,
    widest: Annotated[int, widest_option] = 8,
):
    if pstats and pstats.exists():
        FAILED(f"won't overwrite: {pstats}")

    if unit_tests.problem_count():
        FAILED("unit tests")

    random = Random(deepest=deepest, widest=widest)
    memory = TimedTindalwic(pstats)
    ruamel = None if pstats else TimedRuamel(memory)

    if loops:
        for loop in track(range(loops)):
            separated = memory.separated(random.file())

            if diff_any(separated.file, memory.file(separated.python)):
                FAILED("to python and back")

            with memory.encode(separated.file) as buffer:
                if diff_any(separated, memory.separated(memory.decode(buffer))):
                    FAILED("encode then decode")

            if ruamel:
                yaml = ruamel.translate(separated.file)
                if diff_translate(separated, yaml):
                    FAILED("YAML translate")
                if diff_ruamel(yaml, ruamel.roundtrip(yaml)):
                    FAILED("YAML roundtrip")

        memory.timers()
        if ruamel:
            ruamel.timers()


if __name__ == "__main__":
    app()
