import { cn } from "@lumiforum/ui";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

export function MarkdownContent({ content, className }: { content: string; className?: string }) {
  return (
    <div className={cn("min-w-0 text-[15px] leading-7 text-foreground", className)}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
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
          img: ({ src, alt }) => (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={src}
              alt={alt || ""}
              loading="lazy"
              decoding="async"
              className="my-5 h-auto max-w-full rounded-md border border-border"
            />
          ),
          hr: () => <hr className="my-8 border-border" />,
          table: ({ children }) => (
            <table className="my-5 block w-full overflow-x-auto border-collapse text-sm">
              {children}
            </table>
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
