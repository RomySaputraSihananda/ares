"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import clsx from "clsx";

const links = [
  { href: "/",         label: "Dashboard" },
  { href: "/trades",   label: "Trades"    },
  { href: "/backtest", label: "Backtest"  },
];

export default function Nav() {
  const path = usePathname();
  return (
    <nav className="sticky top-0 z-50" style={{ backgroundColor: 'var(--c-canvas)', borderBottom: '1px solid var(--c-hl)' }}>
      <div className="max-w-5xl mx-auto px-4 sm:px-6 h-14 flex items-center gap-6">
        {/* wordmark */}
        <Link href="/" className="font-mono text-xs tracking-widest text-ink-sub uppercase hover:text-ink transition-colors">
          ARES
        </Link>

        {/* nav links */}
        <div className="flex gap-1">
          {links.map(({ href, label }) => (
            <Link
              key={href}
              href={href}
              className={clsx(
                "px-3 py-1.5 rounded-md text-sm transition-colors",
                path === href
                  ? "bg-s2 text-ink"
                  : "text-ink-sub hover:text-ink hover:bg-s1"
              )}
            >
              {label}
            </Link>
          ))}
        </div>

        {/* live indicator */}
        <div className="ml-auto flex items-center gap-2">
          <span className="w-1.5 h-1.5 rounded-full bg-bull pulse-dot" />
          <span className="text-xs text-ink-sub font-medium tracking-eyebrow uppercase">Live</span>
        </div>
      </div>
    </nav>
  );
}
