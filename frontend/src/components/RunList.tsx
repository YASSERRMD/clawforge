import { useEffect, useRef, useState } from 'react';
import { RefreshCw, AlertCircle } from 'lucide-react';
import type { Event, RunSummary } from '../types';

interface RunListProps {
    onSelectRun: (runId: string) => void;
    liveEvents?: Event[];
}

export function RunList({ onSelectRun, liveEvents = [] }: RunListProps) {
    const [runs, setRuns] = useState<RunSummary[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const lastEventCount = useRef(0);

    const fetchRuns = async () => {
        setLoading(true);
        setError(null);
        try {
            const res = await fetch('/api/runs');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            setRuns(data.runs ?? []);
        } catch (e) {
            setError((e as Error).message ?? 'Failed to load runs');
        } finally {
            setLoading(false);
        }
    };

    // Initial load
    useEffect(() => { fetchRuns(); }, []);

    // Refresh when new run-related events arrive via WebSocket
    useEffect(() => {
        if (liveEvents.length === lastEventCount.current) return;
        lastEventCount.current = liveEvents.length;
        const latest = liveEvents[0];
        if (latest && ['RunStarted', 'RunCompleted', 'RunFailed'].includes(latest.kind as string)) {
            fetchRuns();
        }
    }, [liveEvents]);

    return (
        <div className="bg-white rounded-lg shadow p-4">
            <div className="flex justify-between items-center mb-4">
                <h2 className="text-xl font-bold">Recent Runs</h2>
                <button
                    onClick={fetchRuns}
                    disabled={loading}
                    className="p-2 hover:bg-gray-100 rounded-full disabled:opacity-50"
                    title="Refresh"
                >
                    <RefreshCw className={`w-5 h-5 ${loading ? 'animate-spin' : ''}`} />
                </button>
            </div>

            {error ? (
                <div className="flex flex-col items-center gap-2 py-4 text-sm text-red-600">
                    <AlertCircle className="w-5 h-5" />
                    <span>{error}</span>
                    <button
                        onClick={fetchRuns}
                        className="px-3 py-1 bg-red-50 border border-red-200 rounded hover:bg-red-100"
                    >
                        Retry
                    </button>
                </div>
            ) : (
                <div className="space-y-2">
                    {runs.length === 0 ? (
                        <p className="text-gray-500 text-center py-4">
                            {loading ? 'Loading…' : 'No runs found'}
                        </p>
                    ) : (
                        runs.map((run) => (
                            <div
                                key={run.run_id}
                                onClick={() => onSelectRun(run.run_id)}
                                className="p-3 border rounded hover:bg-gray-50 cursor-pointer transition-colors"
                            >
                                <div className="flex justify-between items-center">
                                    <span className="font-mono text-sm text-gray-700 truncate w-32" title={run.run_id}>
                                        {run.run_id.split('-')[0]}…
                                    </span>
                                    <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                                        run.status === 'run_completed' ? 'bg-green-100 text-green-800' :
                                        run.status === 'run_failed'    ? 'bg-red-100 text-red-800'   :
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
            )}
        </div>
    );
}
