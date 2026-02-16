import { useEffect, useState } from 'react';
import { Play, Plus } from 'lucide-react';
import type { Agent } from '../types';

export function AgentList() {
    const [agents, setAgents] = useState<Agent[]>([]);
    const [loading, setLoading] = useState(false);

    const fetchAgents = async () => {
        try {
            const res = await fetch('http://localhost:3000/api/agents');
            const data = await res.json();
            setAgents(data.agents);
        } catch (error) {
            console.error('Failed to fetch agents:', error);
        }
    };

    const runAgent = async (id: string) => {
        try {
            setLoading(true);
            const res = await fetch(`http://localhost:3000/api/agents/${id}/run`, {
                method: 'POST'
            });
            if (res.ok) {
                alert('Run triggered successfully!');
            } else {
                alert('Failed to trigger run');
            }
        } catch (e) {
            console.error(e);
            alert('Error triggering run');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchAgents();
    }, []);

    return (
        <div className="bg-white rounded-lg shadow p-4">
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
                                disabled={loading}
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
