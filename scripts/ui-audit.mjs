import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "..");
const sveltePath = path.join(root, "src", "routes", "+page.svelte");
const cssPaths = [
  path.join(root, "src", "app.css"),
  path.join(root, "src", "lib", "styles", "workbench.css"),
];

const markup = fs.readFileSync(sveltePath, "utf8");
const css = cssPaths.map((file) => fs.readFileSync(file, "utf8")).join("\n");
const used = new Set();

for (const match of markup.matchAll(/\bclass="([^"]+)"/g)) {
  for (const name of match[1].split(/\s+/).filter(Boolean)) {
    if (/^[A-Za-z_-][A-Za-z0-9_-]*$/.test(name)) used.add(name);
  }
}
for (const match of markup.matchAll(/\bclass:([A-Za-z_-][A-Za-z0-9_-]*)/g)) used.add(match[1]);

const defined = new Set([...css.matchAll(/\.([A-Za-z_-][A-Za-z0-9_-]*)/g)].map((match) => match[1]));
const missing = [...used].filter((name) => !defined.has(name)).sort();
const hoverTooltips = [...markup.matchAll(/\btitle\s*=\s*(?:"[^"]*"|'[^']*'|\{[^}]*\})/g)].map((match) => match[0]);

const failures = [];
if (missing.length) failures.push(`Classes used in markup without CSS: ${missing.join(", ")}`);
if (hoverTooltips.length) failures.push(`Hover title attributes are forbidden by DESIGN.MD: ${hoverTooltips.join(", ")}`);

if (failures.length) {
  console.error("DrvMatch UI audit failed:\n- " + failures.join("\n- "));
  process.exit(1);
}

console.log(`DrvMatch UI audit passed: ${used.size} markup classes are styled and no hover title attributes were found.`);
