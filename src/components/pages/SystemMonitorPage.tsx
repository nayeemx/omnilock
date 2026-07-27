import { useState, useEffect, useRef } from "react";
import {
  Cpu, MemoryStick, Monitor, Wifi, WifiOff,
  ArrowUp, ArrowDown, Clock, Loader2,
} from "lucide-react";
import { getSystemStats, type SystemStats } from "../../lib/tauri-bridge";

function RingGauge({ pct, color, size = 80 }: { pct: number; color: string; size?: number }) {
  const r = (size - 8) / 2;
  const circ = 2 * Math.PI * r;
  const offset = circ - (pct / 100) * circ;
  return (
    <svg width={size} height={size} className="transform -rotate-90">
      <circle cx={size / 2} cy={size / 2} r={r} fill="none"
        stroke="var(--surface)" strokeWidth={6} />
      <circle cx={size / 2} cy={size / 2} r={r} fill="none"
        stroke={color} strokeWidth={6} strokeLinecap="round"
        strokeDasharray={circ} strokeDashoffset={offset}
        style={{ transition: "stroke-dashoffset 0.8s ease" }} />
    </svg>
  );
}

function StatCard({ icon: Icon, label, value, sub, color }: {
  icon: React.ElementType; label: string; value: string; sub?: string; color: string;
}) {
  return (
    <div className="glass rounded-2xl p-5 flex items-center gap-4">
      <div className="w-10 h-10 rounded-xl flex items-center justify-center"
        style={{ background: `${color}20` }}>
        <Icon className="w-5 h-5" style={{ color }} />
      </div>
      <div className="flex-1 min-w-0">
        <div className="text-[11px] uppercase tracking-widest text-[color:var(--muted-foreground)]">{label}</div>
        <div className="text-lg font-semibold mt-0.5 truncate">{value}</div>
        {sub && <div className="text-xs text-[color:var(--muted-foreground)] truncate">{sub}</div>}
      </div>
    </div>
  );
}

function MiniGraph({ data, color, max }: { data: number[]; color: string; max: number }) {
  const w = 200;
  const h = 40;
  const points = data.map((v, i) => {
    const x = (i / (data.length - 1 || 1)) * w;
    const y = h - (Math.min(v, max) / (max || 1)) * h;
    return `${x},${y}`;
  }).join(" ");

  return (
    <svg width={w} height={h} className="w-full h-10">
      <defs>
        <linearGradient id={`grad-${color.replace("#", "")}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.3" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      <polyline points={points} fill="none" stroke={color} strokeWidth="2" strokeLinejoin="round" />
      <polygon
        points={`0,${h} ${points} ${w},${h}`}
        fill={`url(#grad-${color.replace("#", "")})`} />
    </svg>
  );
}

export function SystemMonitorPage() {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [history, setHistory] = useState<{
    cpu: number[]; ram: number[]; netUp: number[]; netDown: number[];
  }>({ cpu: [], ram: [], netUp: [], netDown: [] });
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    const fetchStats = async () => {
      try {
        const s = await getSystemStats();
        setStats(s);
        setHistory(prev => ({
          cpu: [...prev.cpu.slice(-59), s.cpu_usage],
          ram: [...prev.ram.slice(-59), s.ram_usage_pct],
          netUp: [...prev.netUp.slice(-59), s.net_sent_rate],
          netDown: [...prev.netDown.slice(-59), s.net_recv_rate],
        }));
      } catch (e) {
        console.error("Failed to fetch system stats:", e);
      }
    };

    fetchStats();
    intervalRef.current = setInterval(fetchStats, 2000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  if (!stats) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-6 h-6 animate-spin text-[color:var(--primary)]" />
      </div>
    );
  }

  const formatUptime = (secs: number) => {
    const d = Math.floor(secs / 86400);
    const h = Math.floor((secs % 86400) / 3600);
    const m = Math.floor((secs % 3600) / 60);
    if (d > 0) return `${d}d ${h}h ${m}m`;
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  };

  const formatBytes = (mb: number) => {
    if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
    return `${mb.toFixed(0)} MB`;
  };

  const maxNetRate = Math.max(...history.netUp, ...history.netDown, 100);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">System Monitor</h1>
        <p className="text-sm text-[color:var(--muted-foreground)] mt-1">Real-time hardware utilization</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="glass rounded-2xl p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl flex items-center justify-center bg-[color:var(--primary)]/15">
                <Cpu className="w-5 h-5 text-[color:var(--primary)]" />
              </div>
              <div>
                <div className="text-sm font-medium">CPU</div>
                <div className="text-xs text-[color:var(--muted-foreground)] truncate max-w-[180px]">{stats.cpu_name}</div>
              </div>
            </div>
            <div className="relative flex items-center justify-center">
              <RingGauge pct={stats.cpu_usage} color="var(--primary)" size={72} />
              <span className="absolute text-sm font-semibold">{stats.cpu_usage.toFixed(0)}%</span>
            </div>
          </div>
          <div className="text-xs text-[color:var(--muted-foreground)] mb-2">{stats.cpu_cores} cores</div>
          <MiniGraph data={history.cpu} color="#06b6d4" max={100} />
        </div>

        <div className="glass rounded-2xl p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl flex items-center justify-center bg-[color:var(--success)]/15">
                <MemoryStick className="w-5 h-5 text-[color:var(--success)]" />
              </div>
              <div>
                <div className="text-sm font-medium">Memory</div>
                <div className="text-xs text-[color:var(--muted-foreground)]">
                  {formatBytes(stats.ram_used_mb)} / {formatBytes(stats.ram_total_mb)}
                </div>
              </div>
            </div>
            <div className="relative flex items-center justify-center">
              <RingGauge pct={stats.ram_usage_pct} color="var(--success)" size={72} />
              <span className="absolute text-sm font-semibold">{stats.ram_usage_pct.toFixed(0)}%</span>
            </div>
          </div>
          <div className="text-xs text-[color:var(--muted-foreground)] mb-2">
            {formatBytes(stats.ram_total_mb - stats.ram_used_mb)} available
          </div>
          <MiniGraph data={history.ram} color="#22c55e" max={100} />
        </div>

        <div className="glass rounded-2xl p-6">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-10 h-10 rounded-xl flex items-center justify-center bg-[color:var(--warning)]/15">
              <Monitor className="w-5 h-5 text-[color:var(--warning)]" />
            </div>
            <div>
              <div className="text-sm font-medium">GPU</div>
              <div className="text-xs text-[color:var(--muted-foreground)] truncate max-w-[200px]">{stats.gpu_name}</div>
            </div>
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-[color:var(--muted-foreground)]">VRAM</span>
              <span className="font-medium">{stats.gpu_vram_mb > 0 ? formatBytes(stats.gpu_vram_mb) : "N/A"}</span>
            </div>
          </div>
        </div>

        <div className="glass rounded-2xl p-6">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-10 h-10 rounded-xl flex items-center justify-center bg-[color:var(--info)]/15">
              <Wifi className="w-5 h-5 text-[color:var(--info)]" />
            </div>
            <div>
              <div className="text-sm font-medium">Network</div>
              <div className="text-xs text-[color:var(--muted-foreground)]">Live traffic</div>
            </div>
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between items-center">
              <span className="text-[color:var(--muted-foreground)] flex items-center gap-1">
                <ArrowUp className="w-3 h-3 text-[color:var(--success)]" /> Sent
              </span>
              <span className="font-medium">{formatBytes(stats.net_sent_mb)} total</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-[color:var(--muted-foreground)] flex items-center gap-1">
                <ArrowDown className="w-3 h-3 text-[color:var(--primary)]" /> Received
              </span>
              <span className="font-medium">{formatBytes(stats.net_recv_mb)} total</span>
            </div>
            <div className="flex justify-between">
              <span className="text-[color:var(--muted-foreground)]">Upload rate</span>
              <span className="font-medium">{stats.net_sent_rate.toFixed(1)} KB/s</span>
            </div>
            <div className="flex justify-between">
              <span className="text-[color:var(--muted-foreground)]">Download rate</span>
              <span className="font-medium">{stats.net_recv_rate.toFixed(1)} KB/s</span>
            </div>
          </div>
          <div className="mt-3">
            <MiniGraph data={history.netDown} color="#3b82f6" max={maxNetRate} />
          </div>
        </div>
      </div>

      <div className="glass rounded-2xl p-4">
        <div className="flex items-center gap-2 text-sm text-[color:var(--muted-foreground)]">
          <Clock className="w-4 h-4" />
          System uptime: <span className="font-medium text-[color:var(--foreground)]">{formatUptime(stats.uptime_secs)}</span>
        </div>
      </div>
    </div>
  );
}
