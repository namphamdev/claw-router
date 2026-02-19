import { useEffect, useState } from "react";
import api from "../api";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/Card";

export function Settings() {
  const [config, setConfig] = useState<any>(null);

  useEffect(() => {
    api.get("/api/config").then((res) => setConfig(res.data));
  }, []);

  const saveConfig = async () => {
    try {
      await api.post("/api/config", config);
      alert("Config saved!");
    } catch (e) {
      alert("Failed to save config");
    }
  };

  if (!config) return <div className="p-4">Loading config...</div>;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Routing Profile</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex gap-2">
            {config.profiles.map((p: any) => (
              <Button
                key={p.name}
                variant={config.active_profile === p.name ? "primary" : "secondary"}
                onClick={() => setConfig({ ...config, active_profile: p.name })}
              >
                {p.name.toUpperCase()}
              </Button>
            ))}
          </div>
          <p className="mt-2 text-sm text-gray-500">
            {config.profiles.find((p:any) => p.name === config.active_profile)?.description}
          </p>
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-2">
        {config.providers.map((provider: any, idx: number) => (
          <Card key={provider.id}>
             <CardHeader className="pb-2">
                <CardTitle className="text-lg flex justify-between items-center">
                    {provider.name}
                    <input
                        type="checkbox"
                        checked={provider.enabled}
                        onChange={(e) => {
                            const newProviders = [...config.providers];
                            newProviders[idx].enabled = e.target.checked;
                            setConfig({ ...config, providers: newProviders });
                        }}
                        className="h-4 w-4"
                    />
                </CardTitle>
             </CardHeader>
             <CardContent>
                <div className="space-y-2">
                    <label className="text-sm font-medium">API Key</label>
                    <Input
                        type="password"
                        value={provider.api_key || ""}
                        onChange={(e) => {
                            const newProviders = [...config.providers];
                            newProviders[idx].api_key = e.target.value;
                            setConfig({ ...config, providers: newProviders });
                        }}
                        placeholder="sk-..."
                    />
                    <div className="text-xs text-gray-500 uppercase font-semibold">Tier: {provider.tier}</div>
                </div>
             </CardContent>
          </Card>
        ))}
      </div>

      <div className="flex justify-end">
        <Button onClick={saveConfig}>Save Changes</Button>
      </div>
    </div>
  );
}
