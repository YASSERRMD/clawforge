import type { Event } from '../types';
import { Terminal, Clock, Activity, AlertTriangle } from 'lucide-react';

interface EventFeedProps {
    events: Event[];
    title?: string;
}

export function EventFeed({ events, title = "Live Activity" }: EventFeedProps) {
    return (
        <div className="bg-white rounded-lg shadow flex flex-col h-full max-h-[80vh]">
            <div className="p-4 border-b flex items-center gap-2">
                <Activity className="w-5 h-5 text-blue-500" />
                <h2 className="text-xl font-bold">{title}</h2>
                <span className="ml-auto text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded">
                    {events.length} events
                </span>
            </div>

            <div className="flex-1 overflow-auto p-4 space-y-4">
                {events.length === 0 ? (
                    <div className="text-center text-gray-400 py-10 flex flex-col items-center">
                        <Terminal className="w-12 h-12 mb-2 opacity-20" />
                        <p>Waiting for events...</p>
                    </div>
                ) : (
                    events.map((event) => (
                        <div key={event.id} className="flex gap-3 animate-in fade-in slide-in-from-bottom-2 duration-300">
                            <div className="flex-shrink-0 mt-1">
                                {getEventIcon(event.kind)}
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2 mb-1">
                                    <span className="font-semibold text-gray-800 text-sm">{formatKind(event.kind)}</span>
                                    <span className="text-xs text-gray-400 flex items-center gap-1">
                                        <Clock className="w-3 h-3" />
                                        {new Date(event.timestamp).toLocaleTimeString()}
                                    </span>
                                </div>

                                <div className="bg-gray-50 rounded p-2 text-sm font-mono overflow-x-auto border border-gray-100">
                                    <pre className="whitespace-pre-wrap break-words">{JSON.stringify(event.payload, null, 2)}</pre>
                                </div>
                            </div>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}

function getEventIcon(kind: string) {
    if (kind.includes('failed') || kind.includes('error')) return <AlertTriangle className="w-5 h-5 text-red-500" />;
    if (kind.includes('completed')) return <div className="w-2 h-2 rounded-full bg-green-500 mt-1.5" />;
    if (kind.includes('action')) return <Terminal className="w-5 h-5 text-purple-500" />;
    return <div className="w-2 h-2 rounded-full bg-blue-300 mt-1.5" />;
}

function formatKind(kind: string) {
    return kind.split('_').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
}
