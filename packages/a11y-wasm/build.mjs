/**
 * Baut das WASM-Artefakt aus dem Crate `a11y-wasm` nach `pkg/`.
 *
 * Nicht über wasm-pack: das kennt kein `--profile` und baut deshalb immer mit
 * `[profile.release]` des Workspaces (opt-level 3). Größe geht hier vor
 * Geschwindigkeit, dafür gibt es `[profile.wasm-release]` — siehe
 * docs/releasing.md.
 */

import { execFileSync } from "node:child_process";
import { mkdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = dirname(dirname(here));
const pkg = join(here, "pkg");
const target = join(
  repo,
  "target/wasm32-unknown-unknown/wasm-release/a11y_wasm.wasm",
);

const run = (cmd, args, cwd = repo) =>
  execFileSync(cmd, args, { cwd, stdio: "inherit" });

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
run("wasm-bindgen", [
  "--target",
  "web",
  "--out-dir",
  pkg,
  "--out-name",
  "a11y_wasm",
  target,
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
console.log(`a11y_wasm_bg.wasm: ${kib} KiB`);
