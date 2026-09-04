import { test } from "node:test";
import assert from "node:assert/strict";

import { plainText, uniqueKeywords, w3cDate, xmlEscape } from "../src/lib/seo/utils.ts";

test("plainText strips markdown and keeps link text", () => {
  assert.equal(
    plainText("# 标题\n\n**重点** [链接](https://example.com) 结尾"),
    "标题 重点 链接 结尾",
  );
});

test("plainText drops code fences and inline code", () => {
  assert.equal(plainText("文本\n```rust\nfn main() {}\n```\n`code` 继续"), "文本 继续");
});

test("plainText drops images but keeps trailing text", () => {
  assert.equal(plainText("![图](a.png) 继续"), "继续");
});

test("plainText truncates with ellipsis", () => {
  const result = plainText("a".repeat(200), 20);
  assert.equal(result.length, 20);
  assert.ok(result.endsWith("…"));
});

test("uniqueKeywords deduplicates case-insensitively and caps at limit", () => {
  assert.deepEqual(
    uniqueKeywords(["LumiForum", "lumiforum", null, "论坛", undefined, "LUMIFORUM"], 8),
    ["LumiForum", "论坛"],
  );
  assert.deepEqual(uniqueKeywords(["a", "b", "c", "d"], 2), ["a", "b"]);
});

test("xmlEscape escapes all five special characters", () => {
  assert.equal(
    xmlEscape(`<a href="x">'&'</a>`),
    "&lt;a href=&quot;x&quot;&gt;&apos;&amp;&apos;&lt;/a&gt;",
  );
});

test("w3cDate returns ISO format", () => {
  assert.equal(w3cDate("2024-05-01T00:00:00Z").startsWith("2024-05-01T00:00:00"), true);
});
