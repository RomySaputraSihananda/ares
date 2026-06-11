import type { MetadataRoute } from "next";

const BASE = "https://ares-six-peach.vercel.app";

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    { url: BASE,              lastModified: new Date(), changeFrequency: "hourly",  priority: 1.0 },
    { url: `${BASE}/trades`,  lastModified: new Date(), changeFrequency: "hourly",  priority: 0.9 },
    { url: `${BASE}/backtest`,lastModified: new Date(), changeFrequency: "monthly", priority: 0.7 },
  ];
}
