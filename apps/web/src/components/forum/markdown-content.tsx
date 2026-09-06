"use client";

import { cn } from "@lumiforum/ui";
import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

/** Split plain text into segments, highlighting `@username` mentions. */
function highlightMentions(text: string): React.ReactNode[] {
  const parts = text.split(/(@[A-Za-z0-9][A-Za-z0-9_]{2,31})/g);
  return parts.map((part, index) =>
    /^@[A-Za-z0-9][A-Za-z0-9_]{2,31}$/.test(part) ? (
      <span key={index} className="rounded-sm bg-primary/10 px-0.5 font-medium text-primary">
        {part}
      </span>
    ) : (
      part
    ),
  );
}

/**
 * Defers network fetch + decode of user-posted images until they approach
 * the viewport. Long topics embed many large images (uploads are capped at
 * 2560px); eagerly loading all of them competes for bandwidth and slows the
 * topic payload even though most sit far below the fold. An aspect-ratio
 * placeholder (from the optional `w` query param emitted by the editors)
 * keeps layout stable while nothing has loaded.
 */
function LazyImage({ src, alt }: { src: string; alt: string }) {
  const [activated, setActivated] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const holderRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const holder = holderRef.current;
    if (!holder) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setActivated(true);
          observer.disconnect();
        }
      },
      { rootMargin: "400px 0px" },
    );
    observer.observe(holder);
    return () => observer.disconnect();
  }, []);

  // Editors append `?w=<width>` from the upload record when inserting the
  // markdown; only width is used so height scales proportionally.
  const widthParam = (() => {
    try {
      const parsed = new URL(src, "http://placeholder.local");
      const value = Number(parsed.searchParams.get("w"));
      return Number.isFinite(value) && value > 0 ? value : null;
    } catch {
      return null;
    }
  })();

  return (
    <div
      ref={holderRef}
      className="my-5 w-fit max-w-full overflow-hidden rounded-md border border-border"
      style={
        widthParam
          ? { aspectRatio: `${widthParam} / auto` }
          : activated
            ? undefined
            : { minHeight: "1px" }
      }
    >
      {activated ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={src}
          alt={alt}
          loading="lazy"
          decoding="async"
          onLoad={() => setLoaded(true)}
          className={cn(
            "h-auto w-auto max-w-full transition-opacity duration-300",
            loaded ? "opacity-100" : "opacity-0",
          )}
        />
      ) : null}
    </div>
  );
}

export function MarkdownContent({ content, className }: { content: string; className?: string }) {
  return (
    <div className={cn("min-w-0 text-[15px] leading-7 text-foreground", className)}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
          text: ({ children }) => {
            const value = typeof children === "string" ? children : String(children ?? "");
            return <>{highlightMentions(value)}</>;
          },
          h1: ({ children }) => <h1 className="mb-4 mt-8 text-3xl font-semibold">{children}</h1>,
          h2: ({ children }) => (
            <h2 className="mb-3 mt-8 border-b border-border pb-2 text-2xl font-semibold">
              {children}
            </h2>
          ),
          h3: ({ children }) => <h3 className="mb-2 mt-6 text-xl font-semibold">{children}</h3>,
          p: ({ children }) => <p className="my-4 break-words">{children}</p>,
          ul: ({ children }) => <ul className="my-4 list-disc space-y-1 pl-6">{children}</ul>,
          ol: ({ children }) => <ol className="my-4 list-decimal space-y-1 pl-6">{children}</ol>,
          blockquote: ({ children }) => (
            <blockquote className="my-5 border-l-4 border-primary/40 bg-surface px-4 py-1 text-muted-foreground">
              {children}
            </blockquote>
          ),
          code: ({ className: codeClassName, children, ...props }) => (
            <code
              className={cn(
                "rounded-sm bg-muted px-1.5 py-0.5 font-mono text-sm",
                codeClassName?.includes("language-") &&
                  "block overflow-x-auto rounded-md border border-border p-4 leading-6",
              )}
              {...props}
            >
              {children}
            </code>
          ),
          pre: ({ children }) => <pre className="my-5 overflow-x-auto">{children}</pre>,
          a: ({ href, children }) => (
            <a
              href={href}
              className="text-primary underline underline-offset-4"
              rel="noopener noreferrer"
              target={href?.startsWith("http") ? "_blank" : undefined}
            >
              {children}
            </a>
          ),
          img: ({ src, alt }) => {
            const resolved = typeof src === "string" ? src : "";
            if (!resolved) return null;
            return <LazyImage src={resolved} alt={alt || ""} />;
          },
          hr: () => <hr className="my-8 border-border" />,
          table: ({ children }) => (
            // Horizontal scroll on narrow screens; the wrapper keeps the real
            // table layout intact (display:block on the table itself breaks it).
            <div className="my-5 w-full overflow-x-auto">
              <table className="w-full border-collapse text-sm">{children}</table>
            </div>
          ),
          th: ({ children }) => (
            <th className="border border-border bg-muted px-3 py-2 text-left font-semibold">
              {children}
            </th>
          ),
          td: ({ children }) => <td className="border border-border px-3 py-2">{children}</td>,
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
