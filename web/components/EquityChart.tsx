"use client";
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer,
} from "recharts";
import { formatCurrency } from "@/lib/format";

interface DataPoint { date: string; equity: number }

export default function EquityChart({ data, currency = "USD" }: {
  data: DataPoint[];
  currency?: string;
}) {
  if (data.length < 2) {
    return (
      <div className="flex items-center justify-center h-[260px] text-ink-sub text-sm">
        Not enough data to render chart.
      </div>
    );
  }

  const first = data[0].equity;
  const last  = data[data.length - 1].equity;
  const isPos = last >= first;
  const color = isPos ? "#27a644" : "#e5484d";
  const min   = Math.min(...data.map(d => d.equity));
  const max   = Math.max(...data.map(d => d.equity));

  return (
    <ResponsiveContainer width="100%" height={260}>
      <AreaChart data={data} margin={{ top: 4, right: 4, left: 0, bottom: 0 }}>
        <defs>
          <linearGradient id="eq" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%"   stopColor={color} stopOpacity={0.12} />
            <stop offset="100%" stopColor={color} stopOpacity={0}    />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="4 4" stroke="#23252a" vertical={false} />
        <XAxis
          dataKey="date"
          tick={{ fontSize: 11, fill: "#62666d", fontFamily: "JetBrains Mono, monospace" }}
          tickLine={false} axisLine={false}
          interval="preserveStartEnd"
        />
        <YAxis
          domain={[min * 0.997, max * 1.003]}
          tick={{ fontSize: 11, fill: "#62666d", fontFamily: "JetBrains Mono, monospace" }}
          tickLine={false} axisLine={false}
          tickFormatter={v => `$${v.toFixed(0)}`}
          width={60}
        />
        <Tooltip
          formatter={(v: number) => [formatCurrency(v, currency), "Equity"]}
          contentStyle={{
            background: "#141516",
            border: "1px solid #34343a",
            borderRadius: "8px",
            fontSize: "12px",
            fontFamily: "'JetBrains Mono', monospace",
            color: "#f7f8f8",
          }}
          labelStyle={{ color: "#8a8f98", marginBottom: 4 }}
          cursor={{ stroke: "#34343a", strokeWidth: 1 }}
        />
        <Area
          type="monotone"
          dataKey="equity"
          stroke={color}
          strokeWidth={1.5}
          fill="url(#eq)"
          dot={false}
          activeDot={{ r: 3, strokeWidth: 0, fill: color }}
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}
