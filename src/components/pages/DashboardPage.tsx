import { useState, useEffect } from "react";
import {
  Cloud, Droplets, Wind, Thermometer, Loader2,
  Shield, Lock, FolderLock, LayoutGrid,
} from "lucide-react";
import { getWeather, getSystemStats, type WeatherData, type SystemStats } from "../../lib/tauri-bridge";
import { type VaultConfigDto } from "../../lib/tauri-bridge";

function WeatherIcon({ icon, className }: { icon: string; className?: string }) {
  const cls = className || "w-10 h-10";
  switch (icon) {
    case "sun": return <div className={`${cls} rounded-full bg-yellow-400/20 flex items-center justify-center`}>☀️</div>;
    case "cloud-sun": return <div className={`${cls} rounded-full bg-orange-400/20 flex items-center justify-center`}>⛅</div>;
    case "cloud": return <div className={`${cls} rounded-full bg-gray-400/20 flex items-center justify-center`}>☁️</div>;
    case "cloud-drizzle": return <div className={`${cls} rounded-full bg-blue-400/20 flex items-center justify-center`}>🌧️</div>;
    case "snowflake": return <div className={`${cls} rounded-full bg-cyan-400/20 flex items-center justify-center`}>❄️</div>;
    default: return <Cloud className={cls} />;
  }
}

export function DashboardPage({ config }: { config: VaultConfigDto | null; refresh: () => void }) {
  const [weather, setWeather] = useState<WeatherData | null>(null);
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [loadingWeather, setLoadingWeather] = useState(true);
  const [loadingStats, setLoadingStats] = useState(true);

  useEffect(() => {
    getWeather().then(w => { setWeather(w); setLoadingWeather(false); }).catch(() => setLoadingWeather(false));
    getSystemStats().then(s => { setStats(s); setLoadingStats(false); }).catch(() => setLoadingStats(false));
  }, []);

  const lockedCount = config
    ? config.locked_apps.filter(a => a.enabled).length + config.locked_files.length + config.locked_folders.length + config.locked_drives.length
    : 0;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Dashboard</h1>
        <p className="text-sm text-[color:var(--muted-foreground)] mt-1">Overview of your system</p>
      </div>

      <div className="glass rounded-2xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <Cloud className="w-5 h-5 text-[color:var(--primary)]" />
          <span className="text-sm font-medium">Weather</span>
        </div>
        {loadingWeather ? (
          <div className="flex items-center justify-center h-24">
            <Loader2 className="w-5 h-5 animate-spin text-[color:var(--primary)]" />
          </div>
        ) : weather ? (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <WeatherIcon icon={weather.icon} className="w-14 h-14 text-3xl" />
              <div>
                <div className="text-3xl font-semibold">{weather.temp_c}°C</div>
                <div className="text-sm text-[color:var(--muted-foreground)]">{weather.description}</div>
              </div>
            </div>
            <div className="text-right space-y-1 text-sm">
              <div className="text-[color:var(--muted-foreground)] flex items-center gap-1 justify-end">
                <Thermometer className="w-3 h-3" /> Feels like {weather.feels_like_c}°C
              </div>
              <div className="text-[color:var(--muted-foreground)] flex items-center gap-1 justify-end">
                <Droplets className="w-3 h-3" /> {weather.humidity}%
              </div>
              <div className="text-[color:var(--muted-foreground)] flex items-center gap-1 justify-end">
                <Wind className="w-3 h-3" /> {weather.wind_kph} km/h
              </div>
              <div className="text-xs text-[color:var(--muted-foreground)]">{weather.location}</div>
            </div>
          </div>
        ) : (
          <div className="text-sm text-[color:var(--muted-foreground)]">Weather unavailable</div>
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="glass rounded-2xl p-5">
          <div className="flex items-center gap-3 mb-3">
            <div className="w-9 h-9 rounded-xl flex items-center justify-center bg-[color:var(--primary)]/15">
              <Lock className="w-4 h-4 text-[color:var(--primary)]" />
            </div>
            <div className="text-sm font-medium">Protected Items</div>
          </div>
          <div className="text-3xl font-semibold">{lockedCount}</div>
          <div className="text-xs text-[color:var(--muted-foreground)] mt-1">
            {config?.locked_apps.filter(a => a.enabled).length || 0} apps, {config?.locked_files.length || 0} files
          </div>
        </div>

        <div className="glass rounded-2xl p-5">
          <div className="flex items-center gap-3 mb-3">
            <div className="w-9 h-9 rounded-xl flex items-center justify-center bg-[color:var(--success)]/15">
              <Shield className="w-4 h-4 text-[color:var(--success)]" />
            </div>
            <div className="text-sm font-medium">Security</div>
          </div>
          <div className="space-y-1 text-sm mt-2">
            <div className="flex justify-between">
              <span className="text-[color:var(--muted-foreground)]">2FA</span>
              <span className={config?.totp_enabled ? "text-[color:var(--success)]" : "text-[color:var(--muted-foreground)]"}>
                {config?.totp_enabled ? "Enabled" : "Disabled"}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-[color:var(--muted-foreground)]">Cloud Sync</span>
              <span className={config?.cloud_sync_enabled ? "text-[color:var(--success)]" : "text-[color:var(--muted-foreground)]"}>
                {config?.cloud_sync_enabled ? "Active" : "Inactive"}
              </span>
            </div>
          </div>
        </div>

        <div className="glass rounded-2xl p-5">
          <div className="flex items-center gap-3 mb-3">
            <div className="w-9 h-9 rounded-xl flex items-center justify-center bg-[color:var(--warning)]/15">
              <LayoutGrid className="w-4 h-4 text-[color:var(--warning)]" />
            </div>
            <div className="text-sm font-medium">Quick Status</div>
          </div>
          <div className="space-y-1 text-sm mt-2">
            <div className="flex justify-between">
              <span className="text-[color:var(--muted-foreground)]">CPU</span>
              <span className="font-medium">{loadingStats ? "..." : `${stats?.cpu_usage.toFixed(0)}%`}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-[color:var(--muted-foreground)]">RAM</span>
              <span className="font-medium">{loadingStats ? "..." : `${stats?.ram_usage_pct.toFixed(0)}%`}</span>
            </div>
          </div>
        </div>
      </div>

      <div className="glass rounded-2xl p-5">
        <div className="text-sm font-medium mb-3">Locked Drives</div>
        {config?.locked_drives && config.locked_drives.length > 0 ? (
          <div className="flex gap-2 flex-wrap">
            {config.locked_drives.map(d => (
              <div key={d} className="px-3 py-1.5 rounded-lg bg-surface border border-surface-border text-sm font-mono">
                {d}:\
              </div>
            ))}
          </div>
        ) : (
          <div className="text-sm text-[color:var(--muted-foreground)]">No drives locked</div>
        )}
      </div>
    </div>
  );
}
