import React from 'react';

interface CanvasProps {
    runId: string;
    agentId: string;
}

export const Canvas: React.FC<CanvasProps> = ({ runId: _runId, agentId: _agentId }) => {
    return (
        <div className="border border-gray-300 rounded-md p-4 bg-white min-h-[400px]">
            <h3 className="text-lg font-semibold mb-2">Agent Canvas Surface</h3>
            <p className="text-sm text-gray-500 mb-4">
                This is the experimental Agent-to-UI (A2UI) surface.
                Agents can push React components or markdown directly here as part of their response payload.
            </p>
            <div className="flex items-center justify-center h-48 bg-gray-50 border-2 border-dashed border-gray-200 rounded">
                <span className="text-gray-400 font-mono text-sm pl-2">Waiting for A2UI push payload...</span>
            </div>
        </div>
    );
};
