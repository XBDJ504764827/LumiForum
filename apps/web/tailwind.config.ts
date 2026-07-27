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
        primary: {
          DEFAULT: "hsl(var(--lf-primary))",
          foreground: "hsl(var(--lf-primary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--lf-destructive))",
          foreground: "hsl(var(--lf-destructive-foreground))",
        },
        surface: "hsl(var(--lf-surface))",
        accent: "hsl(var(--lf-accent))",
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
