/**
 * Baut das WASM-Artefakt aus dem Crate `html-conform-wasm` nach `pkg/`.
 *
 * Nicht über wasm-pack: das kennt kein `--profile` und baut deshalb immer mit
 * `[profile.release]` des Workspaces (opt-level 3). Größe geht hier vor
 * Geschwindigkeit, dafür gibt es `[profile.wasm-release]` — siehe
 * docs/releasing.md.
 *
 * Die wasm-bindgen-CLI muss **genau** zur Crate-Version passen, sonst bricht
 * der Schritt mit einem Schema-Fehler ab. Welche Version gebraucht wird, steht
 * in Cargo.lock; passt die CLI auf dem PATH nicht, wird die richtige nach
 * `<target>/wasm-tools/` gelegt, ohne die globale Installation anzufassen.
 *
 * Am Ende wird das Größenbudget (gzip) geprüft; Überschreitung bricht den
 * Bau ab.
 */

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Größenbudget in KiB gzip: gemessen 747,2 KiB (765.111 Bytes) bei der ersten
 * Messung (barrierlab#73), dazu rund 2 % Spielraum. Davon trägt das
 * vorübersetzte HTML5-Schema (`html5.rngc`, 217 KiB roh) den größten Teil.
 * Eine Anhebung ist eine Entscheidung, kein Nebeneffekt.
 */
const BUDGET_KIB = 760;

const here = dirname(fileURLToPath(import.meta.url));
const repo = dirname(dirname(here));
// Ein eigenes Zielverzeichnis (CARGO_TARGET_DIR) gilt auch hier.
const target = resolve(repo, process.env.CARGO_TARGET_DIR ?? "target");
const pkg = join(here, "pkg");
const artefakt = join(
  target,
  "wasm32-unknown-unknown/wasm-release/html_conform_wasm.wasm",
);

const run = (cmd, args, cwd = repo) =>
  execFileSync(cmd, args, { cwd, stdio: "inherit" });

/** Die Version, gegen die das Crate gebaut wird. */
function verlangteVersion() {
  const lock = readFileSync(join(repo, "Cargo.lock"), "utf8");
  const treffer = lock.match(
    /\[\[package\]\]\nname = "wasm-bindgen"\nversion = "([^"]+)"/,
  );
  if (!treffer) throw new Error("wasm-bindgen steht nicht in Cargo.lock");
  return treffer[1];
}

function version(bin) {
  try {
    const out = execFileSync(bin, ["--version"], { encoding: "utf8" });
    return out.trim().split(/\s+/).pop();
  } catch {
    return null;
  }
}

/** Eine CLI, deren Version genau passt — notfalls lokal installiert. */
function cli(soll) {
  if (version("wasm-bindgen") === soll) return "wasm-bindgen";

  const eigen = join(target, "wasm-tools/bin/wasm-bindgen");
  if (existsSync(eigen) && version(eigen) === soll) return eigen;

  console.log(
    `wasm-bindgen ${soll} wird gebraucht (PATH: ${version("wasm-bindgen") ?? "keine"}) — wird nach ${join(target, "wasm-tools")} installiert.`,
  );
  run("cargo", [
    "install",
    "wasm-bindgen-cli",
    "--version",
    soll,
    "--locked",
    "--root",
    join(target, "wasm-tools"),
  ]);
  return eigen;
}

const soll = verlangteVersion();

run("cargo", [
  "build",
  "--profile",
  "wasm-release",
  "--target",
  "wasm32-unknown-unknown",
  "-p",
  "html-conform-wasm",
]);

mkdirSync(pkg, { recursive: true });
run(cli(soll), [
  "--target",
  "web",
  "--out-dir",
  pkg,
  "--out-name",
  "html_conform_wasm",
  artefakt,
]);

// wasm-opt ist optional: rustc emittiert bulk-memory, das muss ausdrücklich
// erlaubt werden, sonst schlägt die Validierung fehl.
const wasm = join(pkg, "html_conform_wasm_bg.wasm");
try {
  run("wasm-opt", [
    "-Oz",
    "--converge",
    "--enable-bulk-memory",
    "--enable-nontrapping-float-to-int",
    wasm,
    "-o",
    wasm,
  ]);
} catch {
  console.warn("wasm-opt nicht gefunden oder fehlgeschlagen — ungeoptimiert.");
}

const roh = statSync(wasm).size;
// Das gzip-Programm, nicht node:zlib: das packt rund 3 % schlechter und ergäbe
// eine andere Zahl als `gzip -9` und die Messung in der Doku.
const gzip = execFileSync("gzip", ["-9", "-c"], {
  input: readFileSync(wasm),
  maxBuffer: 64 * 1024 * 1024,
}).length;
console.log(
  `html_conform_wasm_bg.wasm: ${(roh / 1024).toFixed(1)} KiB roh, ${(gzip / 1024).toFixed(1)} KiB gzip, Budget ${BUDGET_KIB} KiB (wasm-bindgen ${soll})`,
);
if (gzip / 1024 > BUDGET_KIB) {
  console.error(
    `Größenbudget überschritten: ${(gzip / 1024).toFixed(1)} KiB gzip > ${BUDGET_KIB} KiB (ohne wasm-opt ist das Artefakt größer).`,
  );
  process.exit(1);
}
