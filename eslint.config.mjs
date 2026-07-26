import eslintConfigPrettier from "eslint-config-prettier/flat";
import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTypeScript from "eslint-config-next/typescript";
import tseslint from "typescript-eslint";

export default defineConfig([
  ...nextVitals.map((config) => ({
    ...config,
    files: ["apps/web/**/*.{ts,tsx}"],
  })),
  ...nextTypeScript.map((config) => ({
    ...config,
    files: ["apps/web/**/*.{ts,tsx}"],
  })),
  ...tseslint.configs.recommended.map((config) => ({
    ...config,
    files: ["packages/**/*.{ts,tsx}"],
  })),
  {
    files: ["**/*.{ts,tsx}"],
    rules: {
      "@typescript-eslint/consistent-type-imports": [
        "error",
        { prefer: "type-imports", fixStyle: "inline-type-imports" },
      ],
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
  eslintConfigPrettier,
  globalIgnores([
    "**/node_modules/**",
    "**/.next/**",
    "**/.turbo/**",
    "**/dist/**",
    "**/build/**",
    "**/coverage/**",
    "target/**",
    "apps/web/next-env.d.ts",
  ]),
]);
