export interface Event {
    id: string;
    run_id: string;
    agent_id: string;
    timestamp: string;
    kind: string;
    payload: any;
}
