import { useEffect, useState } from 'react';
import { RefreshCw } from 'lucide-react';

interface RunSummary {
    run_id: string;
    event_count: number;
    status: string;
}

export function RunList({ onSelectRun }: { onSelectRun: (runId: string) => void }) {
    const [runs, setRuns] = useState<RunSummary[]>([]);
    const [loading, setLoading] = useState(false);

    const fetchRuns = async () => {
        setLoading(true);
        try {
            const res = await fetch('http://localhost:3000/api/runs');
            const data = await res.json();
            setRuns(data.runs);
        } catch (error) {
            console.error('Failed to fetch runs:', error);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchRuns();
    }, []);

    return (
        <div className="bg-white rounded-lg shadow p-4">
            <div className="flex justify-between items-center mb-4">
                <h2 className="text-xl font-bold">Recent Runs</h2>
                <button
                    onClick={fetchRuns}
                    className="p-2 hover:bg-gray-100 rounded-full"
                    title="Refresh"
                >
                    <RefreshCw className={`w-5 h-5 ${loading ? 'animate-spin' : ''}`} />
                </button>
            </div>

            <div className="space-y-2">
                {runs.length === 0 ? (
                    <p className="text-gray-500 text-center py-4">No runs found</p>
                ) : (
                    runs.map((run) => (
                        <div
                            key={run.run_id}
                            onClick={() => onSelectRun(run.run_id)}
                            className="p-3 border rounded hover:bg-gray-50 cursor-pointer transition-colors"
                        >
                            <div className="flex justify-between items-center">
                                <span className="font-mono text-sm text-gray-700 truncate w-32" title={run.run_id}>
                                    {run.run_id.split('-')[0]}...
                                </span>
                                <span className={`px-2 py-1 rounded-full text-xs font-medium ${run.status === 'run_completed' ? 'bg-green-100 text-green-800' :
                                        run.status === 'run_failed' ? 'bg-red-100 text-red-800' :
                                            'bg-blue-100 text-blue-800'
                                    }`}>
                                    {run.status.replace('run_', '')}
                                </span>
                            </div>
                            <div className="text-xs text-gray-400 mt-1">
                                {run.event_count} events
                            </div>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}
