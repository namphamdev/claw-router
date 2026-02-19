import { useEffect, useState, useCallback } from "react";
import api from "../api";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/Card";
import {
  Activity,
  DollarSign,
  Clock,
  CheckCircle,
  XCircle,
  AlertCircle,
  RefreshCw,
} from "lucide-react";
import { Button } from "./ui/Button";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  Legend,
} from "recharts";

interface ProviderStats {
  requests: number;
  successful: number;
  failed: number;
  total_cost: number;
  avg_duration_ms: number;
}

interface ModelStats {
  requests: number;
  total_cost: number;
}

interface RequestLog {
  id: string;
  timestamp: string;
  model: string;
  provider: string | null;
  status: string;
  duration_ms: number;
  estimated_cost: number | null;
  complexity_tier: string | null;
}

interface Stats {
  requests: number;
  successful: number;
  failed: number;
  total_cost: number;
  avg_duration_ms: number;
  active_profile: string;
  providers: Record<string, ProviderStats>;
  models: Record<string, ModelStats>;
  complexity_tiers: Record<string, number>;
  recent_requests: RequestLog[];
}

const TIER_COLORS: Record<string, string> = {
  Simple: "#22c55e",
  Medium: "#3b82f6",
  Complex: "#f59e0b",
  Reasoning: "#ef4444",
};

const PROVIDER_COLORS = [
  "#3b82f6",
  "#8b5cf6",
  "#06b6d4",
  "#f59e0b",
  "#ec4899",
  "#10b981",
  "#f97316",
];

export function Dashboard() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(false);

  const fetchStats = useCallback(async () => {
    try {
      const res = await api.get("/api/stats");
      setStats(res.data);
    } catch (e) {
      console.error("Failed to fetch stats", e);
    }
  }, []);

  useEffect(() => {
    fetchStats();
  }, [fetchStats]);

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(fetchStats, 5000);
    return () => clearInterval(interval);
  }, [autoRefresh, fetchStats]);

  if (!stats) return <div className="p-4">Loading stats...</div>;

  const successRate =
    stats.requests > 0
      ? ((stats.successful / stats.requests) * 100).toFixed(1)
      : "0";

  // Provider bar chart data
  const providerChartData = Object.entries(stats.providers || {}).map(
    ([name, data]) => ({
      name,
      successful: data.successful,
      failed: data.failed,
    })
  );

  // Complexity pie chart data
  const complexityData = Object.entries(stats.complexity_tiers || {})
    .filter(([, count]) => count > 0)
    .map(([tier, count]) => ({
      name: tier,
      value: count,
    }));

  const statusIcon = (status: string) => {
    switch (status) {
      case "success":
        return <CheckCircle className="h-3.5 w-3.5 text-green-500" />;
      case "error":
        return <XCircle className="h-3.5 w-3.5 text-red-500" />;
      case "no_provider":
        return <AlertCircle className="h-3.5 w-3.5 text-yellow-500" />;
      default:
        return <Clock className="h-3.5 w-3.5 text-gray-400" />;
    }
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const formatCost = (cost: number | null) => {
    if (cost === null || cost === 0) return "-";
    if (cost < 0.001) return `$${cost.toFixed(6)}`;
    return `$${cost.toFixed(4)}`;
  };

  const hasData = stats.requests > 0;

  return (
    <div className="space-y-6">
      {/* Header with refresh controls */}
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Dashboard</h1>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1 text-sm text-gray-600">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
              className="h-3.5 w-3.5"
            />
            Auto-refresh
          </label>
          <Button variant="ghost" onClick={fetchStats} className="p-2">
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Total Requests
            </CardTitle>
            <Activity className="h-4 w-4 text-gray-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.requests}</div>
            <p className="text-xs text-gray-500 mt-1">
              {successRate}% success rate
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Success / Failed
            </CardTitle>
            <CheckCircle className="h-4 w-4 text-green-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              <span className="text-green-600">{stats.successful}</span>
              <span className="text-gray-400 mx-1">/</span>
              <span className="text-red-500">{stats.failed}</span>
            </div>
            <p className="text-xs text-gray-500 mt-1">
              Profile: {stats.active_profile}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Cost</CardTitle>
            <DollarSign className="h-4 w-4 text-gray-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              ${(stats.total_cost ?? 0).toFixed(4)}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Avg Latency</CardTitle>
            <Clock className="h-4 w-4 text-gray-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {stats.requests > 0 ? formatDuration(stats.avg_duration_ms) : "-"}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Charts */}
      {hasData && (
        <div className="grid gap-4 md:grid-cols-2">
          {/* Provider Usage Bar Chart */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">
                Provider Usage
              </CardTitle>
            </CardHeader>
            <CardContent>
              {providerChartData.length > 0 ? (
                <ResponsiveContainer width="100%" height={250}>
                  <BarChart data={providerChartData}>
                    <XAxis
                      dataKey="name"
                      tick={{ fontSize: 12 }}
                      interval={0}
                    />
                    <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
                    <Tooltip />
                    <Bar
                      dataKey="successful"
                      stackId="a"
                      fill="#22c55e"
                      name="Successful"
                    />
                    <Bar
                      dataKey="failed"
                      stackId="a"
                      fill="#ef4444"
                      name="Failed"
                    />
                  </BarChart>
                </ResponsiveContainer>
              ) : (
                <div className="h-[250px] flex items-center justify-center text-gray-400 text-sm">
                  No provider data yet
                </div>
              )}
            </CardContent>
          </Card>

          {/* Complexity Distribution Pie Chart */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">
                Complexity Distribution
              </CardTitle>
            </CardHeader>
            <CardContent>
              {complexityData.length > 0 ? (
                <ResponsiveContainer width="100%" height={250}>
                  <PieChart>
                    <Pie
                      data={complexityData}
                      cx="50%"
                      cy="50%"
                      innerRadius={50}
                      outerRadius={90}
                      paddingAngle={2}
                      dataKey="value"
                      label={({ name, percent }: { name?: string; percent?: number }) =>
                        `${name ?? ""} ${((percent ?? 0) * 100).toFixed(0)}%`
                      }
                    >
                      {complexityData.map((entry) => (
                        <Cell
                          key={entry.name}
                          fill={TIER_COLORS[entry.name] || "#94a3b8"}
                        />
                      ))}
                    </Pie>
                    <Tooltip />
                    <Legend />
                  </PieChart>
                </ResponsiveContainer>
              ) : (
                <div className="h-[250px] flex items-center justify-center text-gray-400 text-sm">
                  No complexity data yet (enable scorer)
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {/* Provider Breakdown Table */}
      {hasData && Object.keys(stats.providers || {}).length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">
              Provider Breakdown
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b text-left text-gray-500">
                    <th className="py-2 pr-4 font-medium">Provider</th>
                    <th className="py-2 pr-4 font-medium text-right">
                      Requests
                    </th>
                    <th className="py-2 pr-4 font-medium text-right">
                      Success Rate
                    </th>
                    <th className="py-2 pr-4 font-medium text-right">
                      Avg Duration
                    </th>
                    <th className="py-2 font-medium text-right">Total Cost</th>
                  </tr>
                </thead>
                <tbody>
                  {Object.entries(stats.providers || {}).map(
                    ([name, data], idx) => (
                      <tr key={name} className="border-b last:border-0">
                        <td className="py-2 pr-4 font-medium">
                          <span className="flex items-center gap-2">
                            <span
                              className="w-2 h-2 rounded-full inline-block"
                              style={{
                                backgroundColor:
                                  PROVIDER_COLORS[
                                    idx % PROVIDER_COLORS.length
                                  ],
                              }}
                            />
                            {name}
                          </span>
                        </td>
                        <td className="py-2 pr-4 text-right font-mono text-xs">
                          {data.requests}
                        </td>
                        <td className="py-2 pr-4 text-right font-mono text-xs">
                          {data.requests > 0
                            ? (
                                (data.successful / data.requests) *
                                100
                              ).toFixed(1)
                            : "0"}
                          %
                        </td>
                        <td className="py-2 pr-4 text-right font-mono text-xs">
                          {formatDuration(data.avg_duration_ms)}
                        </td>
                        <td className="py-2 text-right font-mono text-xs">
                          {formatCost(data.total_cost)}
                        </td>
                      </tr>
                    )
                  )}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Recent Activity */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">Recent Activity</CardTitle>
        </CardHeader>
        <CardContent>
          {(stats.recent_requests || []).length > 0 ? (
            <div className="space-y-2">
              {(stats.recent_requests || []).map((req) => (
                <div
                  key={req.id}
                  className="flex items-center gap-3 py-2 border-b last:border-0 text-sm"
                >
                  <div className="flex-shrink-0">{statusIcon(req.status)}</div>
                  <div className="flex-1 min-w-0">
                    <span className="font-mono text-xs">{req.model}</span>
                    <span className="text-gray-400 mx-2">via</span>
                    <span className="text-gray-600">
                      {req.provider || "-"}
                    </span>
                  </div>
                  {req.complexity_tier && (
                    <span
                      className="text-xs px-1.5 py-0.5 rounded-full"
                      style={{
                        backgroundColor:
                          (TIER_COLORS[req.complexity_tier] || "#94a3b8") + "20",
                        color: TIER_COLORS[req.complexity_tier] || "#94a3b8",
                      }}
                    >
                      {req.complexity_tier}
                    </span>
                  )}
                  <span className="text-xs text-gray-500 font-mono flex-shrink-0">
                    {formatDuration(req.duration_ms)}
                  </span>
                  <span className="text-xs text-gray-500 font-mono flex-shrink-0">
                    {formatCost(req.estimated_cost)}
                  </span>
                  <span className="text-xs text-gray-400 flex-shrink-0">
                    {new Date(req.timestamp).toLocaleTimeString()}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-gray-400 text-sm">
              No requests yet. Send requests to the router to see activity here.
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
