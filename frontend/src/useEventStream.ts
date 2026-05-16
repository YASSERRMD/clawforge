import { useEffect, useState, useRef, useCallback } from 'react';
import type { Event } from './types';

const WS_URL = (import.meta.env.VITE_WS_URL as string | undefined)
    ?? `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${window.location.host}/api/ws`;

const MAX_BACKOFF_MS = 30_000;
const OFFLINE_THRESHOLD = 5;

export function useEventStream() {
    const [events, setEvents] = useState<Event[]>([]);
    const [isConnected, setIsConnected] = useState(false);
    const [isOffline, setIsOffline] = useState(false);
    const ws = useRef<WebSocket | null>(null);
    const retryDelay = useRef(1_000);
    const retryTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
    const destroyed = useRef(false);
    const consecutiveFailures = useRef(0);

    const connect = useCallback(() => {
        if (destroyed.current) return;

        const socket = new WebSocket(WS_URL);
        ws.current = socket;

        socket.onopen = () => {
            retryDelay.current = 1_000;
            consecutiveFailures.current = 0;
            setIsConnected(true);
            setIsOffline(false);
        };

        socket.onmessage = (message) => {
            try {
                const event: Event = JSON.parse(message.data as string);
                setEvents((prev) => [event, ...prev].slice(0, 500));
            } catch (e) {
                console.error('Failed to parse event', e);
            }
        };

        socket.onclose = () => {
            setIsConnected(false);
            if (destroyed.current) return;
            consecutiveFailures.current += 1;
            if (consecutiveFailures.current >= OFFLINE_THRESHOLD) {
                setIsOffline(true);
            }
            retryTimer.current = setTimeout(() => {
                retryDelay.current = Math.min(retryDelay.current * 2, MAX_BACKOFF_MS);
                connect();
            }, retryDelay.current);
        };

        socket.onerror = () => {
            // onclose fires after onerror; reconnect is handled there
            socket.close();
        };
    }, []);

    useEffect(() => {
        destroyed.current = false;
        connect();
        return () => {
            destroyed.current = true;
            if (retryTimer.current) clearTimeout(retryTimer.current);
            ws.current?.close();
        };
    }, [connect]);

    return { events, isConnected, isOffline };
}
