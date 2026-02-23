import pprint
import sys
from io import StringIO
from pathlib import Path
from traceback import format_exception
from typing import Annotated, Any, TypeVar

from deepdiff import DeepDiff
from rich.progress import track
from typer import Typer, Exit, Option

from . import TimedTindalwic, unit_tests
from .ruamel import TimedRuamel
from .generate import Random

T = TypeVar("T")


def diff(was: T, now: T) -> bool:
    if was == now:
        return False
    print()
    print(DeepDiff(was, now, verbose_level=2).pretty())
    return True


def FAILED(message: str, report: Path | None = None, **files):
    print(f"FAILED {message}", file=sys.stderr)
    error: BaseException | None = None
    if report:
        old = set(report.iterdir())
        (report / ".gitignore").write_text("*")
        for name, contents in files.items():
            path = report / name.replace("_", ".")
            match contents:
                case str():
                    print(f"  {path}")
                    path.write_text(contents)
                case bytes():
                    print(f"  {path}")
                    path.write_bytes(contents)
                case BaseException():
                    print(f"  {path}")
                    path.write_text("".join(format_exception(contents)))
                    error = contents
                case None:
                    continue
                case _:
                    print(f"  {path} ??? {type(contents)}")
            old.discard(path)
        for it in old:
            if it.name != ".gitignore":
                it.unlink()
    raise error or Exit(code=1)


def pretty(any: Any) -> str:
    buffer = StringIO()
    pprint.pprint(any, buffer, 4, 120, sort_dicts=False)
    return buffer.getvalue()


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
failures_option = Option(
    help="write failure details to this directory",
    exists=True,
    dir_okay=True,
    file_okay=False,
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
    failures: Annotated[Path | None, failures_option] = None,
):
    if pstats and pstats.exists():
        FAILED(f"won't overwrite: {pstats}")

    if unit_tests.problem_count():
        FAILED("unit tests")

    if loops:
        random = Random(deepest=deepest, widest=widest, empties=False)
        memory = TimedTindalwic(pstats)
        ruamel = None if pstats else TimedRuamel(memory)
        empties = 0

        for loop in track(range(loops)):
            original = memory.separated(random.file())

            modified_file = memory.file(original.python)

            if diff(original.file, modified_file):
                FAILED(
                    "to python and back",
                    failures,
                    original_tindalwic=memory.encode(original.file).getvalue(),
                    original_py=pretty(original.file),
                    modified_tindalwic=memory.encode(modified_file).getvalue(),
                    modified_py=pretty(modified_file),
                )

            encoded = memory.encode(original.file).getvalue()
            modified = memory.separated(memory.decode(encoded))

            if diff(original, modified):
                FAILED(
                    "encode then decode",
                    failures,
                    original_tindalwic=memory.encode(original.file).getvalue(),
                    original_py=pretty(original.file),
                    modified_tindalwic=memory.encode(modified.file).getvalue(),
                    modified_py=pretty(modified.file),
                )

            if not ruamel:
                continue
            translated = ruamel.translate(original)
            if not translated:
                empties += 1
                continue

            if ruamel.error or diff(original.comments, translated.comments):
                FAILED(
                    "translation changed comments",
                    failures,
                    original_tindalwic=memory.encode(original.file).getvalue(),
                    original_yaml=original.yaml,
                    original_comments="\n".join(original.comments),
                    translated_py=pretty(translated.ruamel),
                    translated_comments="\n".join(translated.comments),
                    error=ruamel.error,
                )

            roundtrip = ruamel.roundtrip(translated)
            if ruamel.error or diff(translated, roundtrip):
                roundtrip_ruamel = roundtrip.ruamel if roundtrip else ...
                roundtrip_comments = roundtrip.comments if roundtrip else ()
                FAILED(
                    "YAML roundtrip",
                    failures,
                    original_tindalwic=memory.encode(original.file).getvalue(),
                    original_yaml=original.yaml,
                    translated_py=pretty(translated.ruamel),
                    translated_comments="\n".join(translated.comments),
                    roundtrip_py=pretty(roundtrip_ruamel),
                    roundtrip_comments="\n".join(roundtrip_comments),
                    roundtrip_yaml=ruamel.buffer,
                    error=ruamel.error,
                )

        memory.timers()
        if empties == loops:
            FAILED("all the random data contained empties, no ruamel.yaml timing")
        elif ruamel:
            ruamel.timers(empties, loops)


if __name__ == "__main__":
    app()
