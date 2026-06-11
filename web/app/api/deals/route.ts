import { NextResponse } from "next/server";
import { NextRequest } from "next/server";
import { getDeals } from "@/lib/mt5";

export const dynamic = "force-dynamic";

export async function GET(req: NextRequest) {
  const { searchParams } = req.nextUrl;
  const dateFrom = searchParams.get("date_from") ?? "2025-01-01T00:00:00";
  const dateTo   = searchParams.get("date_to")   ?? new Date().toISOString().slice(0, 19);
  const symbol   = searchParams.get("symbol") ?? undefined;
  try {
    const data = await getDeals(dateFrom, dateTo, symbol);
    return NextResponse.json(data);
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 502 });
  }
}
