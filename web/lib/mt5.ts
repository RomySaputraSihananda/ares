// Server-side only — bridge URL never leaves this file

async function apiFetch<T>(path: string): Promise<T> {
  const base    = process.env.MT5_BASE_URL    ?? "http://localhost:8000";
  const apiKey  = process.env.MT5_API_KEY;
  const headers: Record<string, string> = {};
  if (apiKey) headers["X-API-Key"] = apiKey;
  const res = await fetch(`${base}${path}`, {
    headers,
    next: { revalidate: 0 },
  });
  if (!res.ok) throw new Error(`MT5 bridge ${path} → ${res.status}`);
  return res.json();
}

// ── Types ────────────────────────────────────────────────────────────────────

export interface AccountInfo {
  login: number;
  balance: number;
  equity: number;
  profit: number;
  margin: number;
  margin_free: number;
  margin_level: number;
  currency: string;
  leverage: number;
  name: string;
  server: string;
}

export interface Position {
  ticket: number;
  symbol: string;
  type: number; // 0=buy, 1=sell
  volume: number;
  price_open: number;
  price_current: number;
  sl: number;
  tp: number;
  profit: number;
  swap: number;
  comment: string;
  magic: number;
  time: string;
}

export interface PendingOrder {
  ticket: number;
  symbol: string;
  type: number; // 2=BUY_LIMIT, 3=SELL_LIMIT, 4=BUY_STOP, 5=SELL_STOP
  volume_initial: number;
  volume_current: number;
  price_open: number;
  price_current: number;
  sl: number;
  tp: number;
  magic: number;
  comment: string;
  time_setup: string;
}

export interface Deal {
  ticket: number;
  order: number;
  time: string;
  type: number; // 0=buy, 1=sell, 2=balance
  entry: number; // 0=in, 1=out
  symbol: string;
  volume: number;
  price: number;
  commission: number;
  swap: number;
  profit: number;
  comment: string;
  magic: number;
}

// ── Wrappers ──────────────────────────────────────────────────────────────────

interface DataVec<T> { data: T[]; count: number }

export async function getAccount(): Promise<AccountInfo> {
  const w = await apiFetch<DataVec<AccountInfo>>("/account");
  const item = w.data[0];
  if (!item) throw new Error("empty /account response");
  return item;
}

export async function getPositions(): Promise<Position[]> {
  const w = await apiFetch<DataVec<Position>>("/positions");
  return w.data;
}

export async function getOrders(symbol?: string): Promise<PendingOrder[]> {
  const path = symbol ? `/orders?symbol=${encodeURIComponent(symbol)}` : "/orders";
  const w = await apiFetch<DataVec<PendingOrder>>(path);
  return w.data;
}

export async function getDeals(dateFrom: string, dateTo: string, symbol?: string): Promise<Deal[]> {
  let path = `/history/deals?date_from=${encodeURIComponent(dateFrom)}&date_to=${encodeURIComponent(dateTo)}`;
  if (symbol) path += `&symbol=${encodeURIComponent(symbol)}`;
  const w = await apiFetch<DataVec<Deal>>(path);
  return w.data;
}
