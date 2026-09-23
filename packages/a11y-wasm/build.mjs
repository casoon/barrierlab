/**
 * Baut das WASM-Artefakt aus dem Crate `a11y-wasm` nach `pkg/`.
 *
 * Nicht über wasm-pack: das kennt kein `--profile` und baut deshalb immer mit
 * `[profile.release]` des Workspaces (opt-level 3). Größe geht hier vor
 * Geschwindigkeit, dafür gibt es `[profile.wasm-release]` — siehe
 * docs/releasing.md.
 *
 * Die wasm-bindgen-CLI muss **genau** zur Crate-Version passen, sonst bricht
 * der Schritt mit einem Schema-Fehler ab. Welche Version gebraucht wird, steht
 * in Cargo.lock; passt die CLI auf dem PATH nicht, wird die richtige nach
 * `target/wasm-tools/` gelegt, ohne die globale Installation anzufassen.
 */

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = dirname(dirname(here));
const pkg = join(here, "pkg");
const artefakt = join(
  repo,
  "target/wasm32-unknown-unknown/wasm-release/a11y_wasm.wasm",
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

  const eigen = join(repo, "target/wasm-tools/bin/wasm-bindgen");
  if (existsSync(eigen) && version(eigen) === soll) return eigen;

  console.log(
    `wasm-bindgen ${soll} wird gebraucht (PATH: ${version("wasm-bindgen") ?? "keine"}) — wird nach target/wasm-tools/ installiert.`,
  );
  run("cargo", [
    "install",
    "wasm-bindgen-cli",
    "--version",
    soll,
    "--locked",
    "--root",
    join(repo, "target/wasm-tools"),
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
  "a11y-wasm",
]);

mkdirSync(pkg, { recursive: true });
run(cli(soll), [
  "--target",
  "web",
  "--out-dir",
  pkg,
  "--out-name",
  "a11y_wasm",
  artefakt,
]);

// wasm-opt ist optional: rustc emittiert bulk-memory, das muss ausdrücklich
// erlaubt werden, sonst schlägt die Validierung fehl.
const wasm = join(pkg, "a11y_wasm_bg.wasm");
try {
  run("wasm-opt", [
    "-Oz",
    "--enable-bulk-memory",
    "--enable-nontrapping-float-to-int",
    wasm,
    "-o",
    wasm,
  ]);
} catch {
  console.warn("wasm-opt nicht gefunden oder fehlgeschlagen — ungeoptimiert.");
}

const kib = (statSync(wasm).size / 1024).toFixed(1);
console.log(`a11y_wasm_bg.wasm: ${kib} KiB (wasm-bindgen ${soll})`);
