import { Component, type ErrorInfo, type ReactNode } from 'react';
import { AlertTriangle } from 'lucide-react';

interface Props {
    children: ReactNode;
    label?: string;
}

interface State {
    error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
    state: State = { error: null };

    static getDerivedStateFromError(error: Error): State {
        return { error };
    }

    componentDidCatch(error: Error, info: ErrorInfo) {
        console.error(`[ErrorBoundary:${this.props.label ?? 'unknown'}]`, error, info.componentStack);
    }

    render() {
        if (this.state.error) {
            return (
                <div className="flex flex-col items-center justify-center h-full p-8 text-center text-gray-500">
                    <AlertTriangle className="w-10 h-10 text-red-400 mb-3" />
                    <p className="font-semibold text-red-600">
                        {this.props.label ? `${this.props.label} crashed` : 'Something went wrong'}
                    </p>
                    <pre className="mt-2 text-xs text-left bg-red-50 border border-red-100 rounded p-3 max-w-lg overflow-auto">
                        {this.state.error.message}
                    </pre>
                    <button
                        className="mt-4 px-3 py-1 text-sm bg-gray-100 hover:bg-gray-200 rounded"
                        onClick={() => this.setState({ error: null })}
                    >
                        Retry
                    </button>
                </div>
            );
        }
        return this.props.children;
    }
}
