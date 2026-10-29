# @casoon/a11y-wasm

The accessibility rule set of [barrierlab](https://casoon.github.io/barrierlab) as WebAssembly: the
same rules that run at build time and in CI run inside the open page. The package translates, it
does not judge: it takes the tree a collector hands in and returns the report as JSON.

```sh
npm i @casoon/a11y-wasm
```

The package ships the `wasm-bindgen` output for the web target (`pkg/`): `a11y_wasm.js`, the types
and `a11y_wasm_bg.wasm`. Initialise the module with the default export before creating a `Scan`.

The package does not read the DOM. Collecting the tree (shadow DOM, iframes, the moment of capture)
is the collector's job. Which rules run depends on the columns the collector supplies; a missing
layer ends in `NotRun`, never in "passed".

- [Documentation](https://casoon.github.io/barrierlab/docs/packages/a11y-wasm/)
- [Source](https://github.com/casoon/barrierlab/tree/main/packages/a11y-wasm)

## Verify

Releases are built in GitHub Actions and published with npm provenance. The attestation ties the
package on npm to the commit and workflow that built it:

```sh
npm audit signatures
```

## License

MIT
