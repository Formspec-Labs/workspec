import React, { ErrorInfo, ReactNode } from 'react';
import { AlertTriangle, RefreshCw, Home } from 'lucide-react';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null
    };
  }

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught error:', error, errorInfo);
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null });
    window.location.reload();
  };

  public render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;
      return (
        <div className="min-h-screen bg-slate-50 flex items-center justify-center p-6 font-sans">
          <div className="max-w-md w-full bg-white rounded-[32px] shadow-2xl border border-slate-100 p-10 text-center">
            <div className="w-20 h-20 bg-rose-50 rounded-[24px] flex items-center justify-center mx-auto mb-8">
              <AlertTriangle className="w-10 h-10 text-rose-600" />
            </div>
            
            <h1 className="text-2xl font-black text-slate-900 tracking-tight mb-4">
              Something went wrong
            </h1>
            
            <p className="text-slate-500 font-medium leading-relaxed mb-8">
              The application encountered an unexpected error. Our team has been notified.
            </p>

            {this.state.error && (
              <div className="bg-slate-50 rounded-2xl p-4 mb-8 text-left overflow-hidden">
                <p className="text-[10px] font-black text-slate-400 uppercase tracking-widest mb-2">Error Details</p>
                <p className="text-xs font-mono text-rose-700 break-all">
                  {this.state.error.message}
                </p>
              </div>
            )}

            <div className="flex flex-col gap-3">
              <button
                onClick={this.handleReset}
                className="w-full py-4 bg-blue-600 text-white rounded-2xl font-black uppercase tracking-[0.2em] text-xs hover:bg-blue-700 transition-all flex items-center justify-center gap-3 shadow-xl shadow-blue-100 active:scale-95"
              >
                <RefreshCw className="w-5 h-5" />
                Reload Application
              </button>
              
              <button
                onClick={() => window.location.href = '/'}
                className="w-full py-4 bg-white text-slate-900 border border-slate-200 rounded-2xl font-black uppercase tracking-[0.2em] text-xs hover:bg-slate-50 transition-all flex items-center justify-center gap-3 active:scale-95"
              >
                <Home className="w-5 h-5" />
                Return to Dashboard
              </button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
