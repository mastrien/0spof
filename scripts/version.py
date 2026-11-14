"""Versão única do 0spof em todos os pacotes.

    python scripts/version.py                # mostra a versão de cada arquivo
    python scripts/version.py 0.2.0          # grava 0.2.0 em todos
    python scripts/version.py --check v0.2.0 # CI: falha se algum arquivo não tiver essa versão
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
TAURI = ROOT / "crates/spof-desktop/tauri.conf.json"

SEMVER = re.compile(r"^\d+\.\d+\.\d+$")


def read_all() -> dict[str, str]:
    cargo = re.search(
        r'\[workspace\.package\]\s*\nversion = "([^"]+)"',
        CARGO.read_text(encoding="utf-8"),
    )
    tauri = json.loads(TAURI.read_text(encoding="utf-8")).get("version", "?")
    return {
        "Cargo.toml": cargo.group(1) if cargo else "?",
        "tauri.conf.json": tauri,
    }


def write_json_version(path: Path, version: str) -> None:
    data = json.loads(path.read_text(encoding="utf-8"))
    data["version"] = version
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def write_all(version: str) -> None:
    text = CARGO.read_text(encoding="utf-8")
    text = re.sub(
        r'(\[workspace\.package\]\s*\nversion = ")[^"]+(")',
        rf"\g<1>{version}\g<2>",
        text,
    )
    CARGO.write_text(text, encoding="utf-8")
    write_json_version(TAURI, version)


def main(args: list[str]) -> int:
    if not args:
        for name, version in read_all().items():
            print(f"{name:16} {version}")
        return 0

    if args[0] == "--check":
        if len(args) < 2:
            print("Uso: version.py --check vX.Y.Z", file=sys.stderr)
            return 1
        expected = args[1].removeprefix("v")
        wrong = {k: v for k, v in read_all().items() if v != expected}
        for name, version in wrong.items():
            print(f"{name}: {version} (esperado {expected})", file=sys.stderr)
        return 1 if wrong else 0

    version = args[0].removeprefix("v")
    if not SEMVER.match(version):
        print(f"Versão inválida: {version} (use MAJOR.MINOR.PATCH)", file=sys.stderr)
        return 1

    write_all(version)
    for name, value in read_all().items():
        print(f"{name:16} {value}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
