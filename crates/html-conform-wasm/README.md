# html-conform-wasm

The WASM boundary over [`html-conform`](https://crates.io/crates/html-conform). It contains
**no checks of its own**: `check(html)` calls `html_conform::check` and returns the result as an
`a11y-report` `Report`. A missing check belongs in `html-conform`.

```
npm i @casoon/html-conform-wasm   # the prebuilt WASM package
```

The npm package ships the `wasm-bindgen` output (`pkg/`); see
[docs/packages/html-conform-wasm.md](https://github.com/casoon/barrierlab/blob/main/docs/packages/html-conform-wasm.md).

## License

MIT
