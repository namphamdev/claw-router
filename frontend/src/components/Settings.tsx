import { useEffect, useState } from "react";
import api from "../api";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/Card";
import { Plus, Trash2, ChevronDown, ChevronUp, RotateCcw } from "lucide-react";

const PROVIDER_TYPES = ["OpenAI", "Anthropic", "Google", "DeepSeek", "XAI", "CustomOpenAI"];
const TIERS = ["Subscription", "Cheap", "Free", "PayPerRequest"];
const COMPLEXITY_TIERS = ["simple", "medium", "complex", "reasoning"];
const COMPLEXITY_LABELS: Record<string, string> = {
  simple: "Simple",
  medium: "Medium",
  complex: "Complex",
  reasoning: "Reasoning",
};

const WEIGHT_LABELS: Record<string, { label: string; desc: string }> = {
  token_count: { label: "Token Count", desc: "Message length indicator" },
  code_presence: { label: "Code Presence", desc: "Programming keywords" },
  reasoning_markers: { label: "Reasoning Markers", desc: "Formal reasoning terminology" },
  technical_terms: { label: "Technical Terms", desc: "Domain-specific vocabulary" },
  creative_markers: { label: "Creative Markers", desc: "Creative/artistic language" },
  simple_indicators: { label: "Simple Indicators", desc: "Basic question patterns (subtracted)" },
  multi_step_patterns: { label: "Multi-step Patterns", desc: "Sequential instructions" },
  question_complexity: { label: "Question Complexity", desc: "Multiple questions indicator" },
  imperative_verbs: { label: "Imperative Verbs", desc: "Action-oriented language" },
  constraint_count: { label: "Constraint Count", desc: "Optimization/constraint specs" },
  output_format: { label: "Output Format", desc: "Structured output requests" },
  reference_complexity: { label: "Reference Complexity", desc: "External references" },
  negation_complexity: { label: "Negation Complexity", desc: "Logical negations" },
  domain_specificity: { label: "Domain Specificity", desc: "Specialized domains" },
  agentic_task: { label: "Agentic Task", desc: "Multi-step task instructions" },
};

const DEFAULT_SCORER = {
  enabled: true,
  weights: {
    token_count: 0.08,
    code_presence: 0.15,
    reasoning_markers: 0.18,
    technical_terms: 0.10,
    creative_markers: 0.05,
    simple_indicators: 0.02,
    multi_step_patterns: 0.12,
    question_complexity: 0.05,
    imperative_verbs: 0.03,
    constraint_count: 0.04,
    output_format: 0.03,
    reference_complexity: 0.02,
    negation_complexity: 0.01,
    domain_specificity: 0.02,
    agentic_task: 0.04,
  },
  tier_boundaries: {
    simple_upper: 0.0,
    medium_upper: 0.3,
    complex_upper: 0.5,
  },
  token_thresholds: {
    short_upper: 500,
    long_lower: 3000,
  },
  confidence_steepness: 12.0,
  confidence_threshold: 0.7,
  max_tokens_force_complex: 100000,
};

const emptyModel = () => ({
  id: "",
  name: "",
  input_cost_per_1m: 0,
  output_cost_per_1m: 0,
  context_window: 128000,
  supports_vision: false,
  supports_function_calling: false,
});

const emptyProvider = () => ({
  id: "",
  name: "",
  provider_type: "CustomOpenAI" as string,
  api_key: "",
  endpoint: "",
  tier: "Cheap" as string,
  enabled: true,
  priority: 1,
  models: [emptyModel()],
});

export function Settings() {
  const [config, setConfig] = useState<any>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newProvider, setNewProvider] = useState(emptyProvider());
  const [expandedProviders, setExpandedProviders] = useState<Record<number, boolean>>({});

  const [showScorerAdvanced, setShowScorerAdvanced] = useState(false);

  useEffect(() => {
    api.get("/api/config").then((res) => {
      const data = res.data;
      if (!data.scorer) {
        data.scorer = { ...DEFAULT_SCORER };
      }
      if (!data.cache) {
        data.cache = { enabled: false, ttl_seconds: 3600, cache_dir: "cache" };
      }
      setConfig(data);
    });
  }, []);

  const saveConfig = async () => {
    try {
      await api.post("/api/config", config);
      alert("Config saved!");
    } catch (e) {
      alert("Failed to save config");
    }
  };

  const addProvider = () => {
    if (!newProvider.name.trim() || !newProvider.id.trim()) return;
    // Filter out empty models
    const provider = {
      ...newProvider,
      models: newProvider.models.filter((m) => m.id.trim()),
    };
    setConfig({ ...config, providers: [...config.providers, provider] });
    setNewProvider(emptyProvider());
    setShowAddForm(false);
  };

  const removeProvider = (idx: number) => {
    if (!confirm(`Remove provider "${config.providers[idx].name}"?`)) return;
    const newProviders = config.providers.filter((_: any, i: number) => i !== idx);
    setConfig({ ...config, providers: newProviders });
  };

  const toggleExpand = (idx: number) => {
    setExpandedProviders((prev) => ({ ...prev, [idx]: !prev[idx] }));
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

          {/* Model Mapping per Complexity Tier */}
          {(() => {
            const profileIdx = config.profiles.findIndex((p: any) => p.name === config.active_profile);
            const profile = config.profiles[profileIdx];
            if (!profile) return null;
            const mapping = profile.model_mapping || {};
            const updateMapping = (tier: string, field: string, value: string) => {
              const newProfiles = [...config.profiles];
              const newMapping = { ...mapping };
              newMapping[tier] = { ...(newMapping[tier] || { model_id: "", provider_id: "" }), [field]: value };
              newProfiles[profileIdx] = { ...profile, model_mapping: newMapping };
              setConfig({ ...config, profiles: newProfiles });
            };
            return (
              <div className="mt-4 border-t pt-4">
                <h3 className="text-sm font-semibold mb-3">Model Mapping by Complexity Tier</h3>
                <p className="text-xs text-gray-400 mb-3">
                  Map each complexity tier to a specific model. When a request is scored, the router will use the mapped model instead of the requested one.
                </p>
                <div className="space-y-2">
                  {COMPLEXITY_TIERS.map((tier) => (
                    <div key={tier} className="grid gap-2 md:grid-cols-3 items-center">
                      <div className="text-sm font-medium">{COMPLEXITY_LABELS[tier]}</div>
                      <Input
                        value={mapping[tier]?.model_id || ""}
                        onChange={(e) => updateMapping(tier, "model_id", e.target.value)}
                        placeholder="Model ID (e.g. claude-opus-4)"
                      />
                      <Input
                        value={mapping[tier]?.provider_id || ""}
                        onChange={(e) => updateMapping(tier, "provider_id", e.target.value)}
                        placeholder="Provider ID (optional)"
                      />
                    </div>
                  ))}
                </div>
              </div>
            );
          })()}
        </CardContent>
      </Card>

      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Providers</h2>
        <Button
          variant={showAddForm ? "secondary" : "primary"}
          onClick={() => setShowAddForm(!showAddForm)}
          className="flex items-center gap-1"
        >
          <Plus className="h-4 w-4" />
          {showAddForm ? "Cancel" : "Add Provider"}
        </Button>
      </div>

      {showAddForm && (
        <Card className="border-blue-300 border-2">
          <CardHeader className="pb-2">
            <CardTitle className="text-lg">New Provider</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-1">
                  <label className="text-sm font-medium">Provider ID *</label>
                  <Input
                    value={newProvider.id}
                    onChange={(e) => setNewProvider({ ...newProvider, id: e.target.value })}
                    placeholder="my-provider"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-sm font-medium">Display Name *</label>
                  <Input
                    value={newProvider.name}
                    onChange={(e) => setNewProvider({ ...newProvider, name: e.target.value })}
                    placeholder="My Provider"
                  />
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-1">
                  <label className="text-sm font-medium">Provider Type</label>
                  <select
                    value={newProvider.provider_type}
                    onChange={(e) => setNewProvider({ ...newProvider, provider_type: e.target.value })}
                    className="flex h-10 w-full rounded-md border border-gray-300 bg-transparent px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                  >
                    {PROVIDER_TYPES.map((t) => (
                      <option key={t} value={t}>{t}</option>
                    ))}
                  </select>
                </div>
                <div className="space-y-1">
                  <label className="text-sm font-medium">Tier</label>
                  <select
                    value={newProvider.tier}
                    onChange={(e) => setNewProvider({ ...newProvider, tier: e.target.value })}
                    className="flex h-10 w-full rounded-md border border-gray-300 bg-transparent px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                  >
                    {TIERS.map((t) => (
                      <option key={t} value={t}>{t}</option>
                    ))}
                  </select>
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-1">
                  <label className="text-sm font-medium">API Endpoint</label>
                  <Input
                    value={newProvider.endpoint}
                    onChange={(e) => setNewProvider({ ...newProvider, endpoint: e.target.value })}
                    placeholder="https://api.example.com/v1/chat/completions"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-sm font-medium">API Key</label>
                  <Input
                    type="password"
                    value={newProvider.api_key}
                    onChange={(e) => setNewProvider({ ...newProvider, api_key: e.target.value })}
                    placeholder="sk-..."
                  />
                </div>
              </div>

              <div className="space-y-1">
                <label className="text-sm font-medium">Priority (higher = tried first within tier)</label>
                <Input
                  type="number"
                  min={1}
                  max={255}
                  value={newProvider.priority}
                  onChange={(e) => setNewProvider({ ...newProvider, priority: parseInt(e.target.value) || 1 })}
                />
              </div>

              {/* Models */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <label className="text-sm font-semibold">Models</label>
                  <Button
                    variant="ghost"
                    className="text-sm"
                    onClick={() =>
                      setNewProvider({
                        ...newProvider,
                        models: [...newProvider.models, emptyModel()],
                      })
                    }
                  >
                    <Plus className="h-3 w-3 mr-1" /> Add Model
                  </Button>
                </div>
                {newProvider.models.map((model, mIdx) => (
                  <div key={mIdx} className="border rounded-md p-3 space-y-2 bg-gray-50">
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium">Model {mIdx + 1}</span>
                      {newProvider.models.length > 1 && (
                        <button
                          onClick={() =>
                            setNewProvider({
                              ...newProvider,
                              models: newProvider.models.filter((_, i) => i !== mIdx),
                            })
                          }
                          className="text-red-500 hover:text-red-700"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      )}
                    </div>
                    <div className="grid gap-2 md:grid-cols-2">
                      <Input
                        value={model.id}
                        onChange={(e) => {
                          const models = [...newProvider.models];
                          models[mIdx] = { ...models[mIdx], id: e.target.value };
                          setNewProvider({ ...newProvider, models });
                        }}
                        placeholder="Model ID (e.g. gpt-4o)"
                      />
                      <Input
                        value={model.name}
                        onChange={(e) => {
                          const models = [...newProvider.models];
                          models[mIdx] = { ...models[mIdx], name: e.target.value };
                          setNewProvider({ ...newProvider, models });
                        }}
                        placeholder="Display name"
                      />
                    </div>
                    <div className="grid gap-2 md:grid-cols-3">
                      <div className="space-y-1">
                        <label className="text-xs text-gray-500">Input $/1M tokens</label>
                        <Input
                          type="number"
                          step="0.01"
                          value={model.input_cost_per_1m}
                          onChange={(e) => {
                            const models = [...newProvider.models];
                            models[mIdx] = { ...models[mIdx], input_cost_per_1m: parseFloat(e.target.value) || 0 };
                            setNewProvider({ ...newProvider, models });
                          }}
                        />
                      </div>
                      <div className="space-y-1">
                        <label className="text-xs text-gray-500">Output $/1M tokens</label>
                        <Input
                          type="number"
                          step="0.01"
                          value={model.output_cost_per_1m}
                          onChange={(e) => {
                            const models = [...newProvider.models];
                            models[mIdx] = { ...models[mIdx], output_cost_per_1m: parseFloat(e.target.value) || 0 };
                            setNewProvider({ ...newProvider, models });
                          }}
                        />
                      </div>
                      <div className="space-y-1">
                        <label className="text-xs text-gray-500">Context window</label>
                        <Input
                          type="number"
                          value={model.context_window}
                          onChange={(e) => {
                            const models = [...newProvider.models];
                            models[mIdx] = { ...models[mIdx], context_window: parseInt(e.target.value) || 0 };
                            setNewProvider({ ...newProvider, models });
                          }}
                        />
                      </div>
                    </div>
                    <div className="flex gap-4">
                      <label className="flex items-center gap-1 text-xs">
                        <input
                          type="checkbox"
                          checked={model.supports_vision}
                          onChange={(e) => {
                            const models = [...newProvider.models];
                            models[mIdx] = { ...models[mIdx], supports_vision: e.target.checked };
                            setNewProvider({ ...newProvider, models });
                          }}
                        />
                        Vision
                      </label>
                      <label className="flex items-center gap-1 text-xs">
                        <input
                          type="checkbox"
                          checked={model.supports_function_calling}
                          onChange={(e) => {
                            const models = [...newProvider.models];
                            models[mIdx] = { ...models[mIdx], supports_function_calling: e.target.checked };
                            setNewProvider({ ...newProvider, models });
                          }}
                        />
                        Function calling
                      </label>
                    </div>
                  </div>
                ))}
              </div>

              <div className="flex justify-end">
                <Button onClick={addProvider}>Add Provider</Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        {config.providers.map((provider: any, idx: number) => (
          <Card key={provider.id || idx}>
             <CardHeader className="pb-2">
                <CardTitle className="text-lg flex justify-between items-center">
                    <span className="flex items-center gap-2">
                      {provider.name}
                      <span className="text-xs text-gray-400 font-normal">({provider.provider_type})</span>
                    </span>
                    <div className="flex items-center gap-2">
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
                      <button onClick={() => toggleExpand(idx)} className="text-gray-400 hover:text-gray-600">
                        {expandedProviders[idx] ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                      </button>
                      <button onClick={() => removeProvider(idx)} className="text-red-400 hover:text-red-600">
                        <Trash2 className="h-4 w-4" />
                      </button>
                    </div>
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

                    {expandedProviders[idx] && (
                      <div className="space-y-3 pt-2 border-t mt-2">
                        <div className="grid gap-2 md:grid-cols-2">
                          <div className="space-y-1">
                            <label className="text-xs text-gray-500">Provider ID</label>
                            <Input
                              value={provider.id || ""}
                              onChange={(e) => {
                                const newProviders = [...config.providers];
                                newProviders[idx] = { ...newProviders[idx], id: e.target.value };
                                setConfig({ ...config, providers: newProviders });
                              }}
                              placeholder="provider-id"
                            />
                          </div>
                          <div className="space-y-1">
                            <label className="text-xs text-gray-500">Display Name</label>
                            <Input
                              value={provider.name || ""}
                              onChange={(e) => {
                                const newProviders = [...config.providers];
                                newProviders[idx] = { ...newProviders[idx], name: e.target.value };
                                setConfig({ ...config, providers: newProviders });
                              }}
                              placeholder="My Provider"
                            />
                          </div>
                        </div>
                        <div className="grid gap-2 md:grid-cols-2">
                          <div className="space-y-1">
                            <label className="text-xs text-gray-500">Endpoint</label>
                            <Input
                              value={provider.endpoint || ""}
                              onChange={(e) => {
                                const newProviders = [...config.providers];
                                newProviders[idx] = { ...newProviders[idx], endpoint: e.target.value };
                                setConfig({ ...config, providers: newProviders });
                              }}
                              placeholder="https://..."
                            />
                          </div>
                          <div className="space-y-1">
                            <label className="text-xs text-gray-500">Priority</label>
                            <Input
                              type="number"
                              min={1}
                              max={255}
                              value={provider.priority || 1}
                              onChange={(e) => {
                                const newProviders = [...config.providers];
                                newProviders[idx] = { ...newProviders[idx], priority: parseInt(e.target.value) || 1 };
                                setConfig({ ...config, providers: newProviders });
                              }}
                            />
                          </div>
                        </div>
                        <div className="grid gap-2 md:grid-cols-2">
                          <div className="space-y-1">
                            <label className="text-xs text-gray-500">Provider Type</label>
                            <select
                              value={provider.provider_type}
                              onChange={(e) => {
                                const newProviders = [...config.providers];
                                newProviders[idx] = { ...newProviders[idx], provider_type: e.target.value };
                                setConfig({ ...config, providers: newProviders });
                              }}
                              className="flex h-10 w-full rounded-md border border-gray-300 bg-transparent px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                            >
                              {PROVIDER_TYPES.map((t) => (
                                <option key={t} value={t}>{t}</option>
                              ))}
                            </select>
                          </div>
                          <div className="space-y-1">
                            <label className="text-xs text-gray-500">Tier</label>
                            <select
                              value={provider.tier}
                              onChange={(e) => {
                                const newProviders = [...config.providers];
                                newProviders[idx] = { ...newProviders[idx], tier: e.target.value };
                                setConfig({ ...config, providers: newProviders });
                              }}
                              className="flex h-10 w-full rounded-md border border-gray-300 bg-transparent px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                            >
                              {TIERS.map((t) => (
                                <option key={t} value={t}>{t}</option>
                              ))}
                            </select>
                          </div>
                        </div>
                        <div className="space-y-2">
                          <div className="flex items-center justify-between">
                            <label className="text-xs text-gray-500 font-semibold">Models ({provider.models?.length || 0})</label>
                            <Button
                              variant="ghost"
                              className="text-xs"
                              onClick={() => {
                                const newProviders = [...config.providers];
                                newProviders[idx] = {
                                  ...newProviders[idx],
                                  models: [...(newProviders[idx].models || []), emptyModel()],
                                };
                                setConfig({ ...config, providers: newProviders });
                              }}
                            >
                              <Plus className="h-3 w-3 mr-1" /> Add Model
                            </Button>
                          </div>
                          {provider.models?.map((model: any, mIdx: number) => (
                            <div key={mIdx} className="border rounded-md p-3 space-y-2 bg-gray-50">
                              <div className="flex items-center justify-between">
                                <span className="text-xs font-medium">Model {mIdx + 1}</span>
                                <button
                                  onClick={() => {
                                    const newProviders = [...config.providers];
                                    newProviders[idx] = {
                                      ...newProviders[idx],
                                      models: newProviders[idx].models.filter((_: any, i: number) => i !== mIdx),
                                    };
                                    setConfig({ ...config, providers: newProviders });
                                  }}
                                  className="text-red-500 hover:text-red-700"
                                >
                                  <Trash2 className="h-3 w-3" />
                                </button>
                              </div>
                              <div className="grid gap-2 md:grid-cols-2">
                                <Input
                                  value={model.id}
                                  onChange={(e) => {
                                    const newProviders = [...config.providers];
                                    const models = [...newProviders[idx].models];
                                    models[mIdx] = { ...models[mIdx], id: e.target.value };
                                    newProviders[idx] = { ...newProviders[idx], models };
                                    setConfig({ ...config, providers: newProviders });
                                  }}
                                  placeholder="Model ID (e.g. gpt-4o)"
                                />
                                <Input
                                  value={model.name}
                                  onChange={(e) => {
                                    const newProviders = [...config.providers];
                                    const models = [...newProviders[idx].models];
                                    models[mIdx] = { ...models[mIdx], name: e.target.value };
                                    newProviders[idx] = { ...newProviders[idx], models };
                                    setConfig({ ...config, providers: newProviders });
                                  }}
                                  placeholder="Display name"
                                />
                              </div>
                              <div className="grid gap-2 md:grid-cols-3">
                                <div className="space-y-1">
                                  <label className="text-xs text-gray-500">Input $/1M tokens</label>
                                  <Input
                                    type="number"
                                    step="0.01"
                                    value={model.input_cost_per_1m}
                                    onChange={(e) => {
                                      const newProviders = [...config.providers];
                                      const models = [...newProviders[idx].models];
                                      models[mIdx] = { ...models[mIdx], input_cost_per_1m: parseFloat(e.target.value) || 0 };
                                      newProviders[idx] = { ...newProviders[idx], models };
                                      setConfig({ ...config, providers: newProviders });
                                    }}
                                  />
                                </div>
                                <div className="space-y-1">
                                  <label className="text-xs text-gray-500">Output $/1M tokens</label>
                                  <Input
                                    type="number"
                                    step="0.01"
                                    value={model.output_cost_per_1m}
                                    onChange={(e) => {
                                      const newProviders = [...config.providers];
                                      const models = [...newProviders[idx].models];
                                      models[mIdx] = { ...models[mIdx], output_cost_per_1m: parseFloat(e.target.value) || 0 };
                                      newProviders[idx] = { ...newProviders[idx], models };
                                      setConfig({ ...config, providers: newProviders });
                                    }}
                                  />
                                </div>
                                <div className="space-y-1">
                                  <label className="text-xs text-gray-500">Context window</label>
                                  <Input
                                    type="number"
                                    value={model.context_window}
                                    onChange={(e) => {
                                      const newProviders = [...config.providers];
                                      const models = [...newProviders[idx].models];
                                      models[mIdx] = { ...models[mIdx], context_window: parseInt(e.target.value) || 0 };
                                      newProviders[idx] = { ...newProviders[idx], models };
                                      setConfig({ ...config, providers: newProviders });
                                    }}
                                  />
                                </div>
                              </div>
                              <div className="flex gap-4">
                                <label className="flex items-center gap-1 text-xs">
                                  <input
                                    type="checkbox"
                                    checked={model.supports_vision}
                                    onChange={(e) => {
                                      const newProviders = [...config.providers];
                                      const models = [...newProviders[idx].models];
                                      models[mIdx] = { ...models[mIdx], supports_vision: e.target.checked };
                                      newProviders[idx] = { ...newProviders[idx], models };
                                      setConfig({ ...config, providers: newProviders });
                                    }}
                                  />
                                  Vision
                                </label>
                                <label className="flex items-center gap-1 text-xs">
                                  <input
                                    type="checkbox"
                                    checked={model.supports_function_calling}
                                    onChange={(e) => {
                                      const newProviders = [...config.providers];
                                      const models = [...newProviders[idx].models];
                                      models[mIdx] = { ...models[mIdx], supports_function_calling: e.target.checked };
                                      newProviders[idx] = { ...newProviders[idx], models };
                                      setConfig({ ...config, providers: newProviders });
                                    }}
                                  />
                                  Function calling
                                </label>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                </div>
             </CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <span>Response Cache</span>
            <label className="flex items-center gap-2 text-sm font-normal">
              <input
                type="checkbox"
                checked={config.cache?.enabled ?? false}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    cache: { ...config.cache, enabled: e.target.checked },
                  })
                }
                className="h-4 w-4"
              />
              Enabled
            </label>
          </CardTitle>
          <p className="text-sm text-gray-500">
            Cache identical requests to disk to avoid repeated upstream calls. Cached responses are matched by model, messages, and output-affecting parameters.
          </p>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-1">
              <label className="text-sm font-medium">TTL (seconds)</label>
              <p className="text-xs text-gray-400">How long cached responses remain valid</p>
              <Input
                type="number"
                min={0}
                step={60}
                value={config.cache?.ttl_seconds ?? 3600}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    cache: {
                      ...config.cache,
                      ttl_seconds: parseInt(e.target.value) || 0,
                    },
                  })
                }
              />
            </div>
            <div className="space-y-1">
              <label className="text-sm font-medium">Cache Directory</label>
              <p className="text-xs text-gray-400">Relative path where cache files are stored</p>
              <Input
                value={config.cache?.cache_dir ?? "cache"}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    cache: {
                      ...config.cache,
                      cache_dir: e.target.value,
                    },
                  })
                }
                placeholder="cache"
              />
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <span>Scoring Weights</span>
            <div className="flex items-center gap-3">
              <button
                onClick={() => {
                  if (confirm("Reset all scorer settings to defaults?")) {
                    setConfig({ ...config, scorer: { ...DEFAULT_SCORER } });
                  }
                }}
                className="text-gray-400 hover:text-gray-600"
                title="Reset to defaults"
              >
                <RotateCcw className="h-4 w-4" />
              </button>
              <label className="flex items-center gap-2 text-sm font-normal">
                <input
                  type="checkbox"
                  checked={config.scorer?.enabled ?? true}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      scorer: { ...config.scorer, enabled: e.target.checked },
                    })
                  }
                  className="h-4 w-4"
                />
                Enabled
              </label>
            </div>
          </CardTitle>
          <p className="text-sm text-gray-500">
            Configure how request complexity is scored across 15 dimensions. Higher weights increase the dimension's influence on routing.
          </p>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {/* Weights */}
            <div className="space-y-3">
              {Object.entries(WEIGHT_LABELS).map(([key, { label, desc }]) => (
                <div key={key} className="flex items-center gap-3">
                  <div className="w-44 shrink-0">
                    <div className="text-sm font-medium">{label}</div>
                    <div className="text-xs text-gray-400">{desc}</div>
                  </div>
                  <input
                    type="range"
                    min={0}
                    max={0.5}
                    step={0.01}
                    value={config.scorer?.weights?.[key] ?? 0}
                    onChange={(e) => {
                      const val = parseFloat(e.target.value);
                      setConfig({
                        ...config,
                        scorer: {
                          ...config.scorer,
                          weights: { ...config.scorer?.weights, [key]: val },
                        },
                      });
                    }}
                    className="flex-1 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-500"
                  />
                  <Input
                    type="number"
                    step={0.01}
                    min={0}
                    max={1}
                    value={config.scorer?.weights?.[key] ?? 0}
                    onChange={(e) => {
                      const val = parseFloat(e.target.value) || 0;
                      setConfig({
                        ...config,
                        scorer: {
                          ...config.scorer,
                          weights: { ...config.scorer?.weights, [key]: val },
                        },
                      });
                    }}
                    className="w-20 text-center"
                  />
                </div>
              ))}
            </div>

            {/* Tier Boundaries */}
            <div className="border-t pt-4">
              <h3 className="text-sm font-semibold mb-3">Tier Boundaries</h3>
              <p className="text-xs text-gray-400 mb-3">Score thresholds that determine complexity tiers: Simple &lt; simple_upper &lt; Medium &lt; medium_upper &lt; Complex &lt; complex_upper &lt; Reasoning</p>
              <div className="grid gap-4 md:grid-cols-3">
                <div className="space-y-1">
                  <label className="text-xs text-gray-500">Simple upper</label>
                  <Input
                    type="number"
                    step={0.05}
                    value={config.scorer?.tier_boundaries?.simple_upper ?? 0.0}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        scorer: {
                          ...config.scorer,
                          tier_boundaries: {
                            ...config.scorer?.tier_boundaries,
                            simple_upper: parseFloat(e.target.value) || 0,
                          },
                        },
                      })
                    }
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-xs text-gray-500">Medium upper</label>
                  <Input
                    type="number"
                    step={0.05}
                    value={config.scorer?.tier_boundaries?.medium_upper ?? 0.3}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        scorer: {
                          ...config.scorer,
                          tier_boundaries: {
                            ...config.scorer?.tier_boundaries,
                            medium_upper: parseFloat(e.target.value) || 0,
                          },
                        },
                      })
                    }
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-xs text-gray-500">Complex upper</label>
                  <Input
                    type="number"
                    step={0.05}
                    value={config.scorer?.tier_boundaries?.complex_upper ?? 0.5}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        scorer: {
                          ...config.scorer,
                          tier_boundaries: {
                            ...config.scorer?.tier_boundaries,
                            complex_upper: parseFloat(e.target.value) || 0,
                          },
                        },
                      })
                    }
                  />
                </div>
              </div>
            </div>

            {/* Token Thresholds */}
            <div className="border-t pt-4">
              <h3 className="text-sm font-semibold mb-3">Token Thresholds</h3>
              <p className="text-xs text-gray-400 mb-3">Messages shorter than "short upper" get a negative token score; longer than "long lower" get a positive score.</p>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-1">
                  <label className="text-xs text-gray-500">Short upper (tokens)</label>
                  <Input
                    type="number"
                    step={100}
                    value={config.scorer?.token_thresholds?.short_upper ?? 500}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        scorer: {
                          ...config.scorer,
                          token_thresholds: {
                            ...config.scorer?.token_thresholds,
                            short_upper: parseInt(e.target.value) || 0,
                          },
                        },
                      })
                    }
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-xs text-gray-500">Long lower (tokens)</label>
                  <Input
                    type="number"
                    step={100}
                    value={config.scorer?.token_thresholds?.long_lower ?? 3000}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        scorer: {
                          ...config.scorer,
                          token_thresholds: {
                            ...config.scorer?.token_thresholds,
                            long_lower: parseInt(e.target.value) || 0,
                          },
                        },
                      })
                    }
                  />
                </div>
              </div>
            </div>

            {/* Advanced */}
            <div className="border-t pt-4">
              <button
                onClick={() => setShowScorerAdvanced(!showScorerAdvanced)}
                className="flex items-center gap-1 text-sm font-semibold text-gray-600 hover:text-gray-800"
              >
                {showScorerAdvanced ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                Advanced Settings
              </button>
              {showScorerAdvanced && (
                <div className="grid gap-4 md:grid-cols-3 mt-3">
                  <div className="space-y-1">
                    <label className="text-xs text-gray-500">Confidence steepness</label>
                    <Input
                      type="number"
                      step={1}
                      value={config.scorer?.confidence_steepness ?? 12.0}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          scorer: {
                            ...config.scorer,
                            confidence_steepness: parseFloat(e.target.value) || 12.0,
                          },
                        })
                      }
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs text-gray-500">Confidence threshold</label>
                    <Input
                      type="number"
                      step={0.05}
                      min={0}
                      max={1}
                      value={config.scorer?.confidence_threshold ?? 0.7}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          scorer: {
                            ...config.scorer,
                            confidence_threshold: parseFloat(e.target.value) || 0.7,
                          },
                        })
                      }
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs text-gray-500">Max tokens force complex</label>
                    <Input
                      type="number"
                      step={10000}
                      value={config.scorer?.max_tokens_force_complex ?? 100000}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          scorer: {
                            ...config.scorer,
                            max_tokens_force_complex: parseInt(e.target.value) || 100000,
                          },
                        })
                      }
                    />
                  </div>
                </div>
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="flex justify-end">
        <Button onClick={saveConfig}>Save Changes</Button>
      </div>
    </div>
  );
}
