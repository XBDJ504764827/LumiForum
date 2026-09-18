/**
 * Client-side summary fallback for topics created before the server started
 * stripping markdown from summaries (image links rendered as raw source).
 * Mirrors `strip_markdown` in `apps/api/src/services/topic.rs`: every
 * `![alt](url)` becomes an in-place `[图片]` marker (multiple kept in order),
 * link labels keep their text, other syntax is dropped.
 */
export function formatTopicSummary(value: string | null | undefined): string {
  if (!value) return "";
  let output = "";
  let index = 0;
  while (index < value.length) {
    const rest = value.slice(index);
    if (rest.startsWith("```")) {
      const end = rest.indexOf("```", 3);
      index = end === -1 ? value.length : index + 3 + end + 3;
      output += " ";
      continue;
    }
    if (rest.startsWith("![")) {
      const mid = rest.indexOf("](");
      if (mid !== -1) {
        const urlEnd = rest.indexOf(")", mid + 2);
        if (urlEnd !== -1) {
          output += " [图片] ";
          index += urlEnd + 1;
          continue;
        }
      }
    }
    if (rest.startsWith("[")) {
      const mid = rest.indexOf("](");
      if (mid !== -1) {
        const urlEnd = rest.indexOf(")", mid + 2);
        if (urlEnd !== -1) {
          output += `${rest.slice(1, mid)} `;
          index += urlEnd + 1;
          continue;
        }
      }
    }
    const ch = rest[0] ?? "";
    if (ch === "`") {
      output += " ";
    } else if (ch !== "" && "#> *_~-+|=<>".includes(ch)) {
      output += " ";
    } else {
      output += ch;
    }
    index += ch.length;
  }
  const normalized = output.split(/\s+/).join(" ").trim();
  return normalized || "[图片]";
}
