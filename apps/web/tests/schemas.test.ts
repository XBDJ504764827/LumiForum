import { test } from "node:test";
import assert from "node:assert/strict";

import { commentEditorSchema } from "../src/lib/forum/comment-schemas.ts";
import { loginSchema } from "../src/lib/auth/schemas.ts";

test("commentEditorSchema accepts non-empty content", () => {
  const result = commentEditorSchema.safeParse({ content: "  hello  " });
  assert.equal(result.success, true);
  if (result.success) assert.equal(result.data.content, "hello");
});

test("commentEditorSchema rejects empty and overlong content", () => {
  assert.equal(commentEditorSchema.safeParse({ content: "   " }).success, false);
  assert.equal(commentEditorSchema.safeParse({ content: "a".repeat(20_001) }).success, false);
});

test("loginSchema rejects empty credentials", () => {
  assert.equal(loginSchema.safeParse({ identifier: "", password: "" }).success, false);
  assert.equal(loginSchema.safeParse({ identifier: "user", password: "short" }).success, false);
});

test("loginSchema trims identifier", () => {
  const result = loginSchema.safeParse({
    identifier: "  user@example.com  ",
    password: "password123",
  });
  assert.equal(result.success, true);
  if (result.success) assert.equal(result.data.identifier, "user@example.com");
});
