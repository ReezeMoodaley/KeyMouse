// The app and project document share the same guide source.
import { readFile, writeFile } from "node:fs/promises";
const guide = JSON.parse(
  await readFile(new URL("../src/guide.json", import.meta.url), "utf8"),
);
const lines = [`# ${guide.title}`, "", guide.intro, ""];
for (const section of guide.sections) {
  lines.push(`## ${section.title}`, "");
  if (section.page) lines.push(`Settings page: ${section.page}.`, "");
  for (const paragraph of section.paragraphs ?? []) lines.push(paragraph, "");
  if (section.steps) {
    section.steps.forEach((step, i) => lines.push(`${i + 1}. ${step}`));
    lines.push("");
  }
  if (section.table) {
    const row = (cells) =>
      `| ${cells.map((cell) => cell.replaceAll("|", "\\|")).join(" | ")} |`;
    lines.push(
      row(section.table.columns),
      row(section.table.columns.map(() => "---")),
    );
    section.table.rows.forEach((cells) => lines.push(row(cells)));
    lines.push("");
  }
}
await writeFile(
  new URL("../docs/PRECISION_GUIDE.md", import.meta.url),
  lines.join("\n"),
);
