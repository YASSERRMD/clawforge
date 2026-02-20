import { useState } from 'react';
import { Layout, Globe } from 'lucide-react';
import { RunList } from './components/RunList';
import { AgentList } from './components/AgentList';
import { RunDetail } from './components/RunDetail';
import { EventFeed } from './components/EventFeed';
import { useEventStream } from './useEventStream';


function App() {
  const { events: liveEvents, isConnected } = useEventStream();
  const [viewMode, setViewMode] = useState<'live' | 'history'>('live');
  const [activeRunId, setActiveRunId] = useState<string | null>(null);

  const handleSelectRun = async (runId: string) => {
    setActiveRunId(runId);
    setViewMode('history');
  };

  return (
    <div className="min-h-screen bg-gray-100 flex flex-col">
      <header className="bg-white shadow border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Layout className="w-6 h-6 text-indigo-600" />
            <span className="text-xl font-bold text-gray-900">ClawForge</span>
          </div>
          <div className="flex items-center gap-2">
            <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-sm text-gray-500 font-medium">
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
        </div>
      </header>

      <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8 flex gap-6">
        <aside className="w-1/3 flex flex-col gap-4">
          <div
            onClick={() => { setViewMode('live'); setActiveRunId(null); }}
            className={`p-4 bg-white rounded-lg shadow cursor-pointer border-l-4 hover:bg-gray-50 transition-all ${viewMode === 'live' ? 'border-indigo-500 ring-2 ring-indigo-500 ring-opacity-50' : 'border-transparent'}`}
          >
            <div className="flex items-center gap-3">
              <Globe className="w-6 h-6 text-indigo-500" />
              <div>
                <h3 className="font-bold text-gray-900">Live Feed</h3>
                <p className="text-xs text-gray-500">Real-time global events</p>
              </div>
            </div>
          </div>

          <RunList onSelectRun={handleSelectRun} />

          <AgentList />
        </aside>

        <section className="flex-1 h-[calc(100vh-6rem)]">
          {viewMode === 'live' ? (
            <EventFeed
              events={liveEvents}
              title="Live Activity"
            />
          ) : (
            activeRunId && <RunDetail runId={activeRunId} />
          )}
        </section>
      </main>
    </div>
  );
}

export default App;
