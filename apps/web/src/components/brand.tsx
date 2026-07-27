import Link from "next/link";

export function Brand() {
  return (
    <Link href="/" className="inline-flex items-center gap-2 text-sm font-semibold text-foreground">
      <span className="flex size-8 items-center justify-center rounded-md bg-foreground text-white">
        <span className="text-sm font-semibold" aria-hidden="true">
          L
        </span>
      </span>
      <span>LumiForum</span>
    </Link>
  );
}
