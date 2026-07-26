import type { HealthResponse } from "@lumiforum/types";
import { getApiBaseUrl, joinUrl } from "@lumiforum/shared";
import { Button } from "@lumiforum/ui";

async function fetchHealth(): Promise<HealthResponse | null> {
  const base = getApiBaseUrl({ isServer: true });
  try {
    const res = await fetch(joinUrl(base, "/health"), {
      next: { revalidate: 10 },
    });
    if (!res.ok) {
      return null;
    }
    return (await res.json()) as HealthResponse;
  } catch {
    return null;
  }
}

export default async function HomePage() {
  const health = await fetchHealth();

  return (
    <main className="mx-auto flex min-h-full max-w-3xl flex-col gap-8 px-6 py-16">
      <header className="space-y-3">
        <p className="text-sm font-medium text-muted-foreground">Phase 1 foundation</p>
        <h1 className="text-4xl font-semibold tracking-tight">LumiForum</h1>
        <p className="max-w-xl text-base text-muted-foreground">
          Monorepo scaffold is online. Forum features will land in later phases.
        </p>
      </header>

      <section className="rounded-md border border-border bg-white p-5 shadow-sm">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
          API health
        </h2>
        {health ? (
          <dl className="grid gap-2 text-sm">
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">status</dt>
              <dd className="font-mono">{health.status}</dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">service</dt>
              <dd className="font-mono">{health.service}</dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">timestamp</dt>
              <dd className="font-mono">{health.timestamp}</dd>
            </div>
          </dl>
        ) : (
          <p className="text-sm text-muted-foreground">
            API unreachable. Start the stack with{" "}
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
              docker compose up
            </code>
            .
          </p>
        )}
      </section>

      <div>
        <Button type="button" variant="outline">
          Scaffold ready
        </Button>
      </div>
    </main>
  );
}
