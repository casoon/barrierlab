# @casoon/html-conform-wasm

[html-conform](https://casoon.github.io/barrierlab/docs/packages/html-conform/) as WebAssembly:
HTML5 conformance checking, comparable to the Nu Html Checker (vnu), in the browser, in Node and
in Workers. No JVM, no network request. The package translates, it does not judge: the checks
live in the Rust crate.

```sh
npm i @casoon/html-conform-wasm
```

The package ships the `wasm-bindgen` output for the web target (`pkg/`): `html_conform_wasm.js`,
the types and `html_conform_wasm_bg.wasm`. Initialise the module once before the first `check`.

```js
// Browser (the .wasm is fetched next to the module)
import init, { check } from "@casoon/html-conform-wasm";
await init();

// Node
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { check, initSync } from "@casoon/html-conform-wasm";
const wasm = createRequire(import.meta.url).resolve(
  "@casoon/html-conform-wasm/html_conform_wasm_bg.wasm",
);
initSync({ module: readFileSync(wasm) });

const report = check("<!doctype html><title>t</title><ul><span>x</span></ul>");
report.findings; // Finding[] in the shared a11y-report model
```

`check(html)` returns the `Report` of the shared finding model (`a11y-report`): `findings`
(each with `rule_id`, `message`, `outcome`, `severity` and `location.position` with line,
column and byte offset), `inputs` and `rule_runs`. **Look at those two as well:**

- `inputs` lists content the checker did not validate, such as CSS in `<style>`
  (`document_incomplete`). An empty `findings` list is not "fully covered".
- A document nested deeper than 512 levels is not checked. The result has no findings, but a
  `rule_runs` entry `html-conform` with `not_run: "errored"` and a reason. That is not a pass.

`check` throws an `Error` only if the checker itself cannot be set up. The first call is slower
than later ones, see the documentation.

- [Documentation](https://casoon.github.io/barrierlab/docs/packages/html-conform-wasm/)
- [Source](https://github.com/casoon/barrierlab/tree/main/packages/html-conform-wasm)

## Verify

Releases are built in GitHub Actions and published with npm provenance. The attestation ties the
package on npm to the commit and workflow that built it:

```sh
npm audit signatures
```

## License

MIT
