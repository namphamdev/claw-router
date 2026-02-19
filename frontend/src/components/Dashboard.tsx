import { useEffect, useState } from "react";
import api from "../api";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/Card";
import { Activity, DollarSign, Settings } from "lucide-react";

interface Stats {
  requests: number;
  saved_cost: number;
  active_profile: string;
}

export function Dashboard() {
  const [stats, setStats] = useState<Stats | null>(null);

  useEffect(() => {
    const fetchStats = async () => {
        try {
            const res = await api.get("/api/stats");
            setStats(res.data);
        } catch (e) {
            console.error("Failed to fetch stats", e);
        }
    };
    fetchStats();
  }, []);

  if (!stats) return <div className="p-4">Loading stats...</div>;

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Total Requests</CardTitle>
          <Activity className="h-4 w-4 text-gray-500" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">{stats.requests}</div>
        </CardContent>
      </Card>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Saved Cost</CardTitle>
          <DollarSign className="h-4 w-4 text-gray-500" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">${stats.saved_cost}</div>
        </CardContent>
      </Card>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Active Profile</CardTitle>
          <Settings className="h-4 w-4 text-gray-500" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold capitalize">{stats.active_profile}</div>
        </CardContent>
      </Card>
    </div>
  );
}
