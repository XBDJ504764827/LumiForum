import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{ts,tsx}", "../../packages/ui/src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        background: "hsl(var(--lf-background))",
        foreground: "hsl(var(--lf-foreground))",
        muted: {
          DEFAULT: "hsl(var(--lf-muted))",
          foreground: "hsl(var(--lf-muted-foreground))",
        },
        border: "hsl(var(--lf-border))",
        ring: "hsl(var(--lf-ring))",
      },
      borderRadius: {
        lg: "0.5rem",
        md: "0.375rem",
        sm: "0.25rem",
      },
    },
  },
  plugins: [],
};

export default config;
