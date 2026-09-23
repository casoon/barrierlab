# a11y-wasm

The WASM boundary over the shared accessibility rule set. It contains **no rules
of its own**: it binds `a11y-dom`, `a11y-rules`, `accname` and `a11y-report` and
exposes two things — an arena adapter over the tree a collector hands in, and the
`wasm-bindgen` boundary above it.

A missing rule belongs in `a11y-rules`. Duplicating one here would break the
promise that a finding has the same identifier on every surface.

```
cargo add a11y-wasm            # as a Rust dependency
npm i @casoon/a11y-wasm        # as the prebuilt WASM package
```

The npm package ships the `wasm-bindgen` output (`pkg/`); see
[docs/packages/a11y-wasm.md](https://github.com/casoon/barrierlab/blob/main/docs/packages/a11y-wasm.md).

## License

MIT
