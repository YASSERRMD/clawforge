export interface Event {
    id: string;
    run_id: string;
    agent_id: string;
    timestamp: string;
    kind: string;
    payload: any;
}

export type RunState = 'active' | 'paused' | 'awaiting_input' | 'cancelled' | 'completed' | 'failed';

export interface RunSummary {
    run_id: string;
    event_count: number;
    status: string;
}

export interface Agent {
    id: string;
    name: string;
    description: string;
    trigger: any;
}

// Removed duplicate definitions
