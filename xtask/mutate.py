#!/usr/bin/env python3
"""Mutationslauf über einen Katalog gezielter Quelltext-Mutationen.

Jeder Eintrag in `<crate>/tests/mutations/*.toml` ersetzt genau eine Stelle
(`find` → `replace`) in einer Quelldatei des Crates. Der Lauf arbeitet im
Arbeitsbaum selbst, eine Mutation nach der anderen, und stellt die Datei in
jedem Fall wieder her (auch bei Abbruch). Keine Kopie des Repositorys, kein
eigenes Zielverzeichnis, nichts parallel.

Ablauf:

1. Ausgangslauf: das Testziel unverändert bauen und `korpus_faelle` je
   Kennung mit `KORPUS_ID=<id>` laufen lassen. Eine Kennung, deren Fälle
   schon ohne Mutation scheitern, ergibt für alle ihre Einträge `invalid`.
2. Je Eintrag: Mutation anwenden, nur bauen (`--no-run`; Fehler →
   `invalid`), dann `korpus_faelle` mit `KORPUS_ID=<id>` laufen lassen,
   Datei wiederherstellen.

Ergebnis je Eintrag:

- killed     — der Test wird rot (erwartet für `expect = "killed"`)
- survived   — der Test bleibt grün: die Fälle sind zu schwach
- equivalent — bleibt grün, und der Katalog begründet, warum (`expect = "equivalent"`)
- invalid    — Ausgangslauf rot, `find` nicht genau einmal, oder die Mutation kompiliert nicht
- timeout    — Bauen oder Test brauchte länger als `--timeout` Sekunden

Aufruf:

    xtask/mutate.py crates/a11y-rules [--only ID[,ID…]] [--catalog DATEI]
                    [--json ERGEBNIS.json] [--timeout SEKUNDEN]
                    [--lock VERZEICHNIS] [--min-free-gb N]

`--lock` (oder `MUTATE_LOCK`) nimmt vor jedem cargo-Aufruf eine Sperre als
Verzeichnis (`mkdir`, mit `pid`-Datei), wie sie andere Werkzeuge auf
derselben Maschine benutzen; die Wartezeit zählt nicht zur Zeitgrenze.
`--min-free-gb` bricht ab, sobald auf dem Datenträger des Zielverzeichnisses
weniger frei ist. Exit-Code 1, wenn ein Eintrag nicht wie erwartet ausgeht.
"""

import argparse
import contextlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
# Der Test, der die Fälle ausführt; die Gate-Tests derselben Datei laufen nicht mit.
FILTER = "korpus_faelle"


@contextlib.contextmanager
def sperre(lock):
    if not lock:
        yield
        return
    lock = pathlib.Path(lock)
    while True:
        try:
            lock.mkdir(parents=False)
            break
        except FileExistsError:
            try:
                pid = int((lock / "pid").read_text().strip())
                os.kill(pid, 0)
            except (FileNotFoundError, ValueError, PermissionError):
                pass
            except ProcessLookupError:
                shutil.rmtree(lock, ignore_errors=True)
                continue
            time.sleep(2)
    (lock / "pid").write_text(str(os.getpid()))
    try:
        yield
    finally:
        shutil.rmtree(lock, ignore_errors=True)


class Zeit(Exception):
    pass


def cargo(args, lock, timeout, env=None):
    e = dict(os.environ)
    if env:
        e.update(env)
    with sperre(lock):
        try:
            p = subprocess.run(["cargo"] + args, cwd=ROOT, env=e, capture_output=True,
                               text=True, timeout=timeout)
        except subprocess.TimeoutExpired:
            raise Zeit()
    return p.returncode, p.stdout + p.stderr


def frei_gb():
    pfad = pathlib.Path(os.environ.get("CARGO_TARGET_DIR") or ROOT / "target")
    while not pfad.exists():
        pfad = pfad.parent
    return shutil.disk_usage(pfad).free / 1e9


def load(crate: pathlib.Path, catalog):
    paths = [pathlib.Path(catalog)] if catalog else sorted((crate / "tests/mutations").glob("*.toml"))
    entries = []
    for p in paths:
        with open(p, "rb") as fh:
            data = tomllib.load(fh)
        for m in data.get("mutation", []):
            m["_catalog"] = p.name
            entries.append(m)
    return entries


def package_name(crate: pathlib.Path):
    with open(crate / "Cargo.toml", "rb") as fh:
        return tomllib.load(fh)["package"]["name"]


def touch(path: pathlib.Path):
    # Sicher neuer als jeder Fingerabdruck, auch bei grober mtime-Auflösung.
    t = time.time() + 2
    os.utime(path, (t, t))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("crate")
    ap.add_argument("--only")
    ap.add_argument("--catalog")
    ap.add_argument("--json")
    ap.add_argument("--timeout", type=int, default=1800)
    ap.add_argument("--lock", default=os.environ.get("MUTATE_LOCK"))
    ap.add_argument("--min-free-gb", type=float, default=0)
    a = ap.parse_args()

    crate = (ROOT / a.crate).resolve()
    pkg = package_name(crate)
    entries = load(crate, a.catalog)
    if a.only:
        wanted = set(a.only.split(","))
        entries = [m for m in entries if m["id"] in wanted]
    if not entries:
        sys.exit("keine Einträge")

    def platz():
        if a.min_free_gb and frei_gb() < a.min_free_gb:
            sys.exit(f"nur noch {frei_gb():.1f} GB frei (< {a.min_free_gb}); Abbruch")

    # 1. Ausgangslauf.
    rot = set()
    for test in sorted({m.get("test", "korpus") for m in entries}):
        platz()
        rc, out = cargo(["test", "-q", "-p", pkg, "--test", test, "--no-run"], a.lock, a.timeout)
        if rc != 0:
            print(out[-4000:])
            sys.exit(f"--test {test} kompiliert unverändert nicht; Abbruch")
        for id_ in sorted({m["id"] for m in entries if m.get("test", "korpus") == test}):
            rc, out = cargo(["test", "-q", "-p", pkg, "--test", test, "--", FILTER],
                            a.lock, a.timeout, env={"KORPUS_ID": id_})
            if rc != 0:
                rot.add((test, id_))
                print(f"!! Ausgangslauf rot: {id_}", flush=True)
    print(f"Ausgangslauf: {len(rot)} Kennungen rot; {len(entries)} Mutationen", flush=True)

    # 2. Mutationen, nacheinander.
    results = []
    for i, m in enumerate(entries, 1):
        test = m.get("test", "korpus")
        target = crate / m["file"]
        t0 = time.time()
        if (test, m["id"]) in rot:
            status, detail = "invalid", "Ausgangslauf rot"
        else:
            original = target.read_text()
            n = original.count(m["find"])
            if n != 1:
                status, detail = "invalid", f"find kommt {n}-mal vor"
            else:
                platz()
                try:
                    target.write_text(original.replace(m["find"], m["replace"], 1))
                    touch(target)
                    rc, out = cargo(["test", "-q", "-p", pkg, "--test", test, "--no-run"],
                                    a.lock, a.timeout)
                    if rc != 0:
                        err = [l for l in out.splitlines() if l.startswith("error")]
                        status, detail = "invalid", (err[0] if err else "Build scheitert")
                    else:
                        rc, out = cargo(["test", "-q", "-p", pkg, "--test", test, "--", FILTER],
                                        a.lock, a.timeout, env={"KORPUS_ID": m["id"]})
                        if rc != 0:
                            status = "killed"
                            lines = [l.strip() for l in out.splitlines()
                                     if l.strip().startswith(m["id"])]
                            detail = lines[0] if lines else f"rc={rc}"
                        elif m.get("expect") == "equivalent":
                            status, detail = "equivalent", m.get("note", "")
                        else:
                            status, detail = "survived", "Test bleibt grün"
                except Zeit:
                    status, detail = "timeout", f"> {a.timeout}s"
                finally:
                    target.write_text(original)
                    touch(target)
        detail = f"{detail} ({time.time() - t0:.0f}s)"
        expected = m.get("expect", "killed")
        ok = status == expected
        results.append({"id": m["id"], "catalog": m["_catalog"], "file": m["file"],
                        "kind": m.get("kind", ""), "find": m["find"], "replace": m["replace"],
                        "status": status, "expect": expected, "ok": ok, "detail": detail})
        mark = "ok " if ok else "!! "
        print(f"{mark}[{i}/{len(entries)}] {m['id']} ({m.get('kind', '')}): {status} — {detail}",
              flush=True)
        if a.json:
            pathlib.Path(a.json).write_text(json.dumps(results, indent=1, ensure_ascii=False))

    counts = {}
    for r in results:
        counts[r["status"]] = counts.get(r["status"], 0) + 1
    print("Summe:", ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    sys.exit(0 if all(r["ok"] for r in results) else 1)


if __name__ == "__main__":
    main()
