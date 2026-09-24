# a11y-perception

What is perceivable on a page — as a snapshot, as a reading-order projection, and
as the difference between two snapshots.

This crate **computes, it does not collect**. It opens no page, speaks no Chrome
DevTools Protocol and reads no DOM. A host hands in an accessibility tree with
focus state; this crate answers what follows from it:

| Building block | The question it answers |
|---|---|
| `AXTree`, `AXNode` | what is in the tree, and how do I find a node again |
| `AXSnapshot`, `FocusSnapshot` | what a given moment looked like |
| `AXTreeDiff` | what an interaction changed *perceivably* |
| `linearize`, `ReadingItem` | in which order a screen reader would encounter it |

## Limits

The projection approximates over browser data. It does not prove what a screen
reader announces: a live region receiving content means the browser had cause to
announce — not that NVDA read it out.

Two snapshots do not carry every case either. A transient change — an error
message inserted, announced and removed again — may never be visible between two
moments. For that a host needs an event trail over time.

Node identity holds **within one document**, via `backend_dom_node_id`. Missing
id, frame switch or navigation means the identity is unclear — and that gets
reported, not guessed.

## Origin

The code ran in [auditmysite](https://github.com/casoon/auditmysite) first and
was measured against 133 pages with 692 journey instances. Details:
[docs/packages/a11y-perception.md](https://github.com/casoon/barrierlab/blob/main/docs/packages/a11y-perception.md).

## License

MIT
