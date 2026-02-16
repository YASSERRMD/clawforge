import { useEffect, useState, useRef } from 'react';
import type { Event } from './types';

export function useEventStream() {
    const [events, setEvents] = useState<Event[]>([]);
    const [isConnected, setIsConnected] = useState(false);
    const ws = useRef<WebSocket | null>(null);

    useEffect(() => {
        // Connect to WebSocket
        // In dev, Vite proxies /api to backend (we need to configure proxy or use absolute URL)
        // For now, assume backend on port 3000
        const wsUrl = 'ws://localhost:3000/api/ws';
        const socket = new WebSocket(wsUrl);

        socket.onopen = () => {
            console.log('Connected to WebSocket');
            setIsConnected(true);
        };

        socket.onmessage = (message) => {
            try {
                const event: Event = JSON.parse(message.data);
                setEvents((prev) => [event, ...prev].slice(0, 100)); // Keep last 100
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
