// Braucht das gebaute `pkg/` (node build.mjs).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { before, test } from "node:test";
import { check, initSync } from "../pkg/html_conform_wasm.js";

before(() => {
  initSync({
    module: readFileSync(
      new URL("../pkg/html_conform_wasm_bg.wasm", import.meta.url),
    ),
  });
});

const page = (body) =>
  `<!doctype html><html lang="en"><head><title>t</title></head><body>${body}</body></html>`;

test("a valid document has no findings and nothing marked as not run", () => {
  const report = check(page("<p>Hello</p>"));
  assert.equal(report.schema, 1);
  assert.deepEqual(report.findings, []);
  assert.equal(report.summary.fail, 0);
  assert.equal(report.inputs ?? undefined, undefined);
  assert.equal(report.rule_runs ?? undefined, undefined);
});

test("an invalid document yields findings in the shared model, with position", () => {
  const report = check(page('<p bogus="1">x</p><ul><span>y</span></ul>'));
  assert.ok(report.findings.length >= 2);
  assert.ok(report.summary.fail >= 2);
  for (const finding of report.findings) {
    assert.ok(finding.rule_id);
    assert.ok(finding.message);
    assert.equal(finding.outcome, "fail");
    const position = finding.location?.position;
    assert.ok(position.line >= 1 && position.column >= 1);
    assert.ok(position.byte_offset >= 0);
  }
  assert.ok(report.findings.some((f) => f.rule_id === "schema.html5"));
});

test("unchecked content is visible in the result, not a silent pass", () => {
  const report = check(
    '<!doctype html><html lang="en"><head><title>t</title><style>p { color: red }</style></head><body><p>x</p></body></html>',
  );
  assert.deepEqual(report.findings, []);
  assert.equal(report.inputs.length, 1);
  assert.equal(report.inputs[0].kind, "document_incomplete");
  assert.equal(report.inputs[0].subject, "style-element-css");
});

test("a document that is too deeply nested is reported as not run", () => {
  const report = check(page("<div>".repeat(600)));
  assert.deepEqual(report.findings, []);
  assert.equal(report.rule_runs.length, 1);
  const [run] = report.rule_runs;
  assert.equal(run.rule_id, "html-conform");
  assert.equal(run.not_run, "errored");
  assert.match(run.reason, /nesting depth/);
});

test("repeated checks give the same result", () => {
  const html = page("<ul><span>y</span></ul>");
  assert.deepEqual(check(html), check(html));
});
