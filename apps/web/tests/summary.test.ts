import { test } from "node:test";
import assert from "node:assert/strict";

import { formatTopicSummary } from "../src/lib/forum/summary.ts";

test("formatTopicSummary keeps one [图片] marker per image, in place", () => {
  assert.equal(
    formatTopicSummary(
      "![1ca4670d.png](https://chat.iquankz.cn/topic_image/2026/08/x.png) 正文 ![b.png](https://x/y.png)",
    ),
    "[图片] 正文 [图片]",
  );
});

test("formatTopicSummary falls back to [图片] for image-only summaries", () => {
  assert.equal(formatTopicSummary("![a.png](https://x/a.png)"), "[图片]");
});

test("formatTopicSummary keeps link labels and strips headings", () => {
  assert.equal(
    formatTopicSummary("# 标题 [LumiForum](https://example.com) 好用"),
    "标题 LumiForum 好用",
  );
});

test("formatTopicSummary leaves clean text untouched", () => {
  assert.equal(formatTopicSummary("纯文字摘要"), "纯文字摘要");
  assert.equal(formatTopicSummary(null), "");
});
