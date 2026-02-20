import { useEffect, useState } from 'react';
import type { Event } from '../types';
import { EventFeed } from './EventFeed';
import { XCircle, MessageSquare } from 'lucide-react';
import { Canvas } from './Canvas';

interface RunDetailProps {
    runId: string;
}

export function RunDetail({ runId }: RunDetailProps) {
    const [events, setEvents] = useState<Event[]>([]);
    const [status, setStatus] = useState<string>('unknown');
    const [inputPrompt, setInputPrompt] = useState<string | null>(null);
    const [inputValue, setInputValue] = useState('');
    const [loading, setLoading] = useState(false);

    const fetchDetails = async () => {
        try {
            const res = await fetch(`http://localhost:3000/api/runs/${runId}`);
            if (res.ok) {
                const data = await res.json();
                setEvents(data.events || []);
                setStatus(data.status);

                // Check for awaiting input
                // Currently data.status is just the last event kind. 
                // We need the *actual* run state from Supervisor.
                // HACK: For now, if last event is "RequestInput", we assume awaiting input.
                // Ideally API should return top-level state.

                // TODO: Update backend to return true state
            }
        } catch (e) {
            console.error(e);
        }
    };

    const cancelRun = async () => {
        if (!confirm('Stop this run?')) return;
        try {
            await fetch(`http://localhost:3000/api/runs/${runId}/cancel`, { method: 'POST' });
            fetchDetails();
        } catch (e) {
            console.error(e);
        }
    };

    const submitInput = async () => {
        if (!inputValue) return;
        setLoading(true);
        try {
            await fetch(`http://localhost:3000/api/runs/${runId}/input`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ input: inputValue })
            });
            setInputValue('');
            setInputPrompt(null);
            fetchDetails();
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        // Poll for updates in history view (live view handled by App.tsx)
        fetchDetails();
        const interval = setInterval(fetchDetails, 2000);
        return () => clearInterval(interval);
    }, [runId]);

    return (
        <div className="flex flex-col h-full bg-white rounded-lg shadow overflow-hidden">
            <div className="p-4 border-b flex justify-between items-center bg-gray-50">
                <div>
                    <h2 className="text-lg font-bold flex items-center gap-2">
                        Run: <span className="font-mono text-sm bg-gray-200 px-2 py-1 rounded">{runId.slice(0, 8)}</span>
                    </h2>
                    <div className="text-sm text-gray-500 capitalize">Status: {status}</div>
                </div>
                <div className="flex gap-2">
                    {status !== 'RunCompleted' && status !== 'RunFailed' && status !== 'Cancelled' && (
                        <button
                            onClick={cancelRun}
                            className="flex items-center gap-1 px-3 py-1 bg-red-100 text-red-700 rounded hover:bg-red-200"
                        >
                            <XCircle className="w-4 h-4" /> Stop
                        </button>
                    )}
                </div>
            </div>

            {inputPrompt && (
                <div className="p-4 bg-yellow-50 border-b border-yellow-200">
                    <div className="flex items-center gap-2 text-yellow-800 font-medium mb-2">
                        <MessageSquare className="w-4 h-4" /> Agent Requesting Input
                    </div>
                    <p className="text-sm text-yellow-700 mb-2">{inputPrompt}</p>
                    <div className="flex gap-2">
                        <input
                            type="text"
                            className="flex-1 border rounded px-3 py-2"
                            value={inputValue}
                            onChange={e => setInputValue(e.target.value)}
                            placeholder="Enter your response..."
                            onKeyDown={e => e.key === 'Enter' && submitInput()}
                        />
                        <button
                            onClick={submitInput}
                            disabled={loading}
                            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
                        >
                            Send
                        </button>
                    </div>
                </div>
            )}

            <div className="flex-1 overflow-auto flex">
                <div className="w-1/2 p-4 border-r border-gray-200">
                    <EventFeed events={events} title="" />
                </div>
                <div className="w-1/2 p-4 bg-gray-50">
                    <Canvas runId={runId} agentId="unknown" />
                </div>
            </div>
        </div>
    );
}
