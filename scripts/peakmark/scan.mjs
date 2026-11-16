// A scan of a synthetic page, handed to the module as the flat columns the browser collector
// would produce. No DOM is needed: `new Scan(...)` takes arrays, as the package documents.
const enc = new TextEncoder();
const tags = ["html", "body", "main", "section", "h2", "p", "a", "img", "button", "input", "label"];
const attrs = ["href", "src", "alt", "id", "for", "type"];
const tagId = Object.fromEntries(tags.map((t, i) => [t, i]));
const attrId = Object.fromEntries(attrs.map((a, i) => [a, i]));

const chunks = [];
let blobLen = 0;
const put = (text) => {
  const bytes = enc.encode(text);
  chunks.push(bytes);
  const at = blobLen;
  blobLen += bytes.length;
  return [at, bytes.length];
};

const nodes = [];
const add = (tag, parent, text = "", attributes = []) => {
  nodes.push({ tag: tagId[tag], parent, text: put(text), attributes: attributes.map(([n, v]) => [attrId[n], put(v)]) });
  return nodes.length - 1;
};

const html = add("html", -1);
const body = add("body", html);
const main = add("main", body);
for (let i = 0; i < 60; i++) {
  const section = add("section", main);
  add("h2", section, `Section ${i}`);
  add("p", section, `Paragraph ${i} with some text that a reader would see.`);
  add("a", section, "read more", [["href", `/page/${i}`]]);
  add("img", section, "", [["src", `/img/${i}.png`]]); // no alt: the scan has something to report
  add("button", section, "");
  add("label", section, `Field ${i}`, [["for", `f${i}`]]);
  add("input", section, "", [["id", `f${i}`], ["type", "text"]]);
}

const attrStart = new Uint32Array(nodes.length + 1);
const attrName = [];
const attrOff = [];
const attrLen = [];
nodes.forEach((node, i) => {
  attrStart[i] = attrName.length;
  for (const [name, [off, len]] of node.attributes) {
    attrName.push(name);
    attrOff.push(off);
    attrLen.push(len);
  }
});
attrStart[nodes.length] = attrName.length;

const blob = new Uint8Array(blobLen);
let at = 0;
for (const chunk of chunks) {
  blob.set(chunk, at);
  at += chunk.length;
}

export default function (a11y) {
  const scan = new a11y.Scan(
    Uint32Array.from(nodes, (n) => n.tag),
    Int32Array.from(nodes, (n) => n.parent),
    Uint32Array.from(nodes, (n) => n.text[0]),
    Uint32Array.from(nodes, (n) => n.text[1]),
    attrStart,
    Uint32Array.from(attrName),
    Uint32Array.from(attrOff),
    Uint32Array.from(attrLen),
    blob,
    tags.join("\u0001"),
    attrs.join("\u0001"),
  );
  const report = scan.run();
  if (scan.nodeCount !== nodes.length) throw new Error("node count differs");
  if (!report.findings.length) throw new Error("the scan reported nothing for a page with images without alt");
  scan.free();
}
