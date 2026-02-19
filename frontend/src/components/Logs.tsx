import { useEffect, useState, useCallback } from "react";
import api from "../api";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/Card";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input";
import {
  RefreshCw,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  ChevronUp,
  CheckCircle,
  XCircle,
  AlertCircle,
  Clock,
} from "lucide-react";

interface RequestLog {
  id: string;
  timestamp: string;
  model: string;
  provider: string | null;
  status: string;
  status_code: number | null;
  duration_ms: number;
  input_tokens: number | null;
  output_tokens: number | null;
  estimated_cost: number | null;
  complexity_tier: string | null;
  complexity_score: number | null;
  error_message: string | null;
  providers_tried: string[];
}

interface LogsResponse {
  logs: RequestLog[];
  total: number;
  limit: number;
  offset: number;
}

const PAGE_SIZE = 25;

export function Logs() {
  const [data, setData] = useState<LogsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(0);
  const [statusFilter, setStatusFilter] = useState<string>("");
  const [modelFilter, setModelFilter] = useState("");
  const [providerFilter, setProviderFilter] = useState("");
  const [expandedRow, setExpandedRow] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(false);

  const fetchLogs = useCallback(async () => {
    try {
      const params: Record<string, string | number> = {
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
      };
      if (statusFilter) params.status = statusFilter;
      if (modelFilter) params.model = modelFilter;
      if (providerFilter) params.provider = providerFilter;

      const res = await api.get("/api/logs", { params });
      setData(res.data);
    } catch (e) {
      console.error("Failed to fetch logs", e);
    } finally {
      setLoading(false);
    }
  }, [page, statusFilter, modelFilter, providerFilter]);

  useEffect(() => {
    setLoading(true);
    fetchLogs();
  }, [fetchLogs]);

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(fetchLogs, 3000);
    return () => clearInterval(interval);
  }, [autoRefresh, fetchLogs]);

  const totalPages = data ? Math.ceil(data.total / PAGE_SIZE) : 0;

  const statusIcon = (status: string) => {
    switch (status) {
      case "success":
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case "error":
        return <XCircle className="h-4 w-4 text-red-500" />;
      case "no_provider":
        return <AlertCircle className="h-4 w-4 text-yellow-500" />;
      default:
        return <Clock className="h-4 w-4 text-gray-400" />;
    }
  };

  const statusBadge = (status: string) => {
    const colors: Record<string, string> = {
      success: "bg-green-100 text-green-800",
      error: "bg-red-100 text-red-800",
      no_provider: "bg-yellow-100 text-yellow-800",
    };
    return (
      <span
        className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${
          colors[status] || "bg-gray-100 text-gray-800"
        }`}
      >
        {statusIcon(status)}
        {status}
      </span>
    );
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const formatCost = (cost: number | null) => {
    if (cost === null) return "-";
    if (cost < 0.001) return `$${cost.toFixed(6)}`;
    return `$${cost.toFixed(4)}`;
  };

  const formatTime = (ts: string) => {
    const d = new Date(ts);
    return d.toLocaleString();
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CardTitle>Request Logs</CardTitle>
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
              <Button
                variant="ghost"
                onClick={() => fetchLogs()}
                className="p-2"
              >
                <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {/* Filters */}
          <div className="flex flex-wrap gap-3 mb-4">
            <div className="flex-1 min-w-[140px]">
              <select
                value={statusFilter}
                onChange={(e) => { setStatusFilter(e.target.value); setPage(0); }}
                className="flex h-9 w-full rounded-md border border-gray-300 bg-transparent px-3 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
              >
                <option value="">All statuses</option>
                <option value="success">Success</option>
                <option value="error">Error</option>
                <option value="no_provider">No provider</option>
              </select>
            </div>
            <div className="flex-1 min-w-[140px]">
              <Input
                value={modelFilter}
                onChange={(e) => { setModelFilter(e.target.value); setPage(0); }}
                placeholder="Filter by model..."
                className="h-9"
              />
            </div>
            <div className="flex-1 min-w-[140px]">
              <Input
                value={providerFilter}
                onChange={(e) => { setProviderFilter(e.target.value); setPage(0); }}
                placeholder="Filter by provider..."
                className="h-9"
              />
            </div>
          </div>

          {/* Table */}
          {loading && !data ? (
            <div className="text-center py-8 text-gray-500">Loading logs...</div>
          ) : data && data.logs.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              No request logs yet. Logs will appear here when requests are made to the router.
            </div>
          ) : data ? (
            <>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-left text-gray-500">
                      <th className="py-2 pr-3 font-medium w-8"></th>
                      <th className="py-2 pr-3 font-medium">Time</th>
                      <th className="py-2 pr-3 font-medium">Status</th>
                      <th className="py-2 pr-3 font-medium">Model</th>
                      <th className="py-2 pr-3 font-medium">Provider</th>
                      <th className="py-2 pr-3 font-medium text-right">Duration</th>
                      <th className="py-2 pr-3 font-medium text-right">Tokens</th>
                      <th className="py-2 font-medium text-right">Cost</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.logs.map((log) => (
                      <>
                        <tr
                          key={log.id}
                          className="border-b hover:bg-gray-50 cursor-pointer"
                          onClick={() => setExpandedRow(expandedRow === log.id ? null : log.id)}
                        >
                          <td className="py-2 pr-3">
                            {expandedRow === log.id ? (
                              <ChevronUp className="h-3.5 w-3.5 text-gray-400" />
                            ) : (
                              <ChevronDown className="h-3.5 w-3.5 text-gray-400" />
                            )}
                          </td>
                          <td className="py-2 pr-3 text-gray-600 whitespace-nowrap">
                            {formatTime(log.timestamp)}
                          </td>
                          <td className="py-2 pr-3">{statusBadge(log.status)}</td>
                          <td className="py-2 pr-3 font-mono text-xs">{log.model}</td>
                          <td className="py-2 pr-3">{log.provider || "-"}</td>
                          <td className="py-2 pr-3 text-right font-mono text-xs">
                            {formatDuration(log.duration_ms)}
                          </td>
                          <td className="py-2 pr-3 text-right font-mono text-xs">
                            {log.input_tokens !== null && log.output_tokens !== null
                              ? `${log.input_tokens} / ${log.output_tokens}`
                              : "-"}
                          </td>
                          <td className="py-2 text-right font-mono text-xs">
                            {formatCost(log.estimated_cost)}
                          </td>
                        </tr>
                        {expandedRow === log.id && (
                          <tr key={`${log.id}-detail`} className="bg-gray-50">
                            <td colSpan={8} className="px-4 py-3">
                              <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-xs">
                                <div>
                                  <span className="text-gray-500 block">Request ID</span>
                                  <span className="font-mono">{log.id.slice(0, 8)}...</span>
                                </div>
                                <div>
                                  <span className="text-gray-500 block">HTTP Status</span>
                                  <span>{log.status_code ?? "-"}</span>
                                </div>
                                <div>
                                  <span className="text-gray-500 block">Complexity Tier</span>
                                  <span>{log.complexity_tier ?? "-"}</span>
                                </div>
                                <div>
                                  <span className="text-gray-500 block">Complexity Score</span>
                                  <span>{log.complexity_score !== null ? log.complexity_score.toFixed(3) : "-"}</span>
                                </div>
                                <div>
                                  <span className="text-gray-500 block">Providers Tried</span>
                                  <span>{log.providers_tried.length > 0 ? log.providers_tried.join(" -> ") : "-"}</span>
                                </div>
                                <div>
                                  <span className="text-gray-500 block">Input Tokens</span>
                                  <span>{log.input_tokens ?? "-"}</span>
                                </div>
                                <div>
                                  <span className="text-gray-500 block">Output Tokens</span>
                                  <span>{log.output_tokens ?? "-"}</span>
                                </div>
                                {log.error_message && (
                                  <div className="col-span-2 md:col-span-4">
                                    <span className="text-gray-500 block">Error</span>
                                    <span className="text-red-600">{log.error_message}</span>
                                  </div>
                                )}
                              </div>
                            </td>
                          </tr>
                        )}
                      </>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* Pagination */}
              <div className="flex items-center justify-between mt-4 text-sm text-gray-600">
                <span>
                  Showing {data.offset + 1}-{Math.min(data.offset + data.logs.length, data.total)} of{" "}
                  {data.total} logs
                </span>
                <div className="flex items-center gap-2">
                  <Button
                    variant="ghost"
                    onClick={() => setPage((p) => Math.max(0, p - 1))}
                    disabled={page === 0}
                    className="p-2"
                  >
                    <ChevronLeft className="h-4 w-4" />
                  </Button>
                  <span>
                    Page {page + 1} of {Math.max(1, totalPages)}
                  </span>
                  <Button
                    variant="ghost"
                    onClick={() => setPage((p) => p + 1)}
                    disabled={page + 1 >= totalPages}
                    className="p-2"
                  >
                    <ChevronRight className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </>
          ) : null}
        </CardContent>
      </Card>
    </div>
  );
}
