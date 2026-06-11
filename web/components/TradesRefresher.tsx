"use client";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";

export default function TradesRefresher() {
  const router = useRouter();
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date());

  useEffect(() => {
    const id = setInterval(() => {
      router.refresh();
      setLastUpdated(new Date());
    }, 30_000);
    return () => clearInterval(id);
  }, [router]);

  return (
    <span className="text-xs text-ink-ter font-mono">
      Updated {lastUpdated.toLocaleTimeString()}
    </span>
  );
}
