import { useEffect, useState } from 'react';
import { Play, Plus, CheckCircle, XCircle } from 'lucide-react';
import type { Agent } from '../types';

type Toast = { id: number; kind: 'success' | 'error'; message: string };

export function AgentList() {
    const [agents, setAgents] = useState<Agent[]>([]);
    const [loading, setLoading] = useState<string | null>(null); // agent id being triggered
    const [toasts, setToasts] = useState<Toast[]>([]);

    const addToast = (kind: Toast['kind'], message: string) => {
        const id = Date.now();
        setToasts((prev) => [...prev, { id, kind, message }]);
        setTimeout(() => setToasts((prev) => prev.filter((t) => t.id !== id)), 3500);
    };

    const fetchAgents = async () => {
        try {
            const res = await fetch('/api/agents');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            setAgents(data.agents ?? []);
        } catch (error) {
            console.error('Failed to fetch agents:', error);
        }
    };

    const runAgent = async (id: string) => {
        setLoading(id);
        try {
            const res = await fetch(`/api/agents/${id}/run`, { method: 'POST' });
            if (res.ok) {
                addToast('success', 'Run triggered successfully');
            } else {
                const body = await res.json().catch(() => ({}));
                addToast('error', (body as { message?: string }).message ?? 'Failed to trigger run');
            }
        } catch (e) {
            console.error(e);
            addToast('error', 'Network error — could not trigger run');
        } finally {
            setLoading(null);
        }
    };

    useEffect(() => { fetchAgents(); }, []);

    return (
        <div className="bg-white rounded-lg shadow p-4 relative">
            {/* Inline toast stack */}
            {toasts.length > 0 && (
                <div className="absolute top-2 right-2 space-y-1 z-10 w-64">
                    {toasts.map((t) => (
                        <div
                            key={t.id}
                            className={`flex items-center gap-2 px-3 py-2 rounded shadow text-sm text-white ${t.kind === 'success' ? 'bg-green-600' : 'bg-red-600'}`}
                        >
                            {t.kind === 'success'
                                ? <CheckCircle className="w-4 h-4 shrink-0" />
                                : <XCircle className="w-4 h-4 shrink-0" />}
                            {t.message}
                        </div>
                    ))}
                </div>
            )}

            <div className="flex justify-between items-center mb-4">
                <h2 className="text-xl font-bold">Agents</h2>
                <button className="p-2 hover:bg-gray-100 rounded-full" title="Create Agent">
                    <Plus className="w-5 h-5" />
                </button>
            </div>

            <div className="space-y-2">
                {agents.length === 0 ? (
                    <p className="text-gray-500 text-center py-4">No agents found</p>
                ) : (
                    agents.map((agent) => (
                        <div key={agent.id} className="p-3 border rounded hover:bg-gray-50 flex justify-between items-center">
                            <div>
                                <div className="font-semibold">{agent.name}</div>
                                <div className="text-xs text-gray-500 truncate w-32">{agent.description}</div>
                            </div>
                            <button
                                onClick={() => runAgent(agent.id)}
                                disabled={loading === agent.id}
                                className="p-2 text-green-600 hover:bg-green-50 rounded-full disabled:opacity-50"
                                title="Run Agent"
                            >
                                <Play className="w-4 h-4" />
                            </button>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}
