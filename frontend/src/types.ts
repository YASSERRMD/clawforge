export type RunState = 'active' | 'paused' | 'awaiting_input' | 'cancelled' | 'completed' | 'failed';

export interface Event {
    id: string;
    run_id: string;
    agent_id: string;
    timestamp: string;
    kind: string;
    payload: Record<string, unknown>;
}

export interface RunSummary {
    run_id: string;
    event_count: number;
    status: string;
}

export interface RunDetail {
    run_id: string;
    status: string;
    events: Event[];
}

export type TriggerKind =
    | { type: 'cron'; expression: string }
    | { type: 'interval'; seconds: number }
    | { type: 'webhook'; path: string }
    | { type: 'manual' };

export interface Agent {
    id: string;
    name: string;
    description: string;
    trigger: TriggerKind;
}
