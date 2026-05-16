import { useEffect, useState, useRef } from 'react';
import type { Event } from './types';

export function useEventStream() {
    const [events, setEvents] = useState<Event[]>([]);
    const [isConnected, setIsConnected] = useState(false);
    const ws = useRef<WebSocket | null>(null);

    useEffect(() => {
        const wsUrl = import.meta.env.VITE_WS_URL as string | undefined
            ?? `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${window.location.host}/api/ws`;
        const socket = new WebSocket(wsUrl);

        socket.onopen = () => {
            console.log('Connected to WebSocket');
            setIsConnected(true);
        };

        socket.onmessage = (message) => {
            try {
                const event: Event = JSON.parse(message.data);
                setEvents((prev) => [event, ...prev].slice(0, 500));
            } catch (e) {
                console.error('Failed to parse event', e);
            }
        };

        socket.onclose = () => {
            console.log('Disconnected from WebSocket');
            setIsConnected(false);
        };

        ws.current = socket;

        return () => {
            socket.close();
        };
    }, []);

    return { events, isConnected };
}
