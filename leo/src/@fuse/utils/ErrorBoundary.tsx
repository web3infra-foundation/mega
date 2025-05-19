import { Component, ErrorInfo, ReactNode } from 'react';

interface ErrorBoundaryProps {
	children?: ReactNode;
}

interface ErrorBoundaryState {
	hasError: boolean;
	error: Error | null;
	errorInfo: ErrorInfo | null;
}

class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
	constructor(props: ErrorBoundaryProps) {
		super(props);
		this.state = { hasError: false, error: null, errorInfo: null };
	}

	static getDerivedStateFromError(error: Error): ErrorBoundaryState {
		// Update state so the next render will show the fallback UI.
		return { hasError: true, error, errorInfo: null };
	}

	componentDidCatch(error: Error, errorInfo: ErrorInfo) {
		// You can also log the error to an error reporting service
		this.setState({ error, errorInfo });

		console.error('Uncaught error:', error, errorInfo);
	}

	render() {
		const { children = null } = this.props;
		const { error, errorInfo, hasError } = this.state;

		if (hasError) {
			return (
				<div className="bg-white p-6">
					<h1 className="text-2xl font-semibold">Something went wrong.</h1>
					<p className="text-base whitespace-pre-wrap">
						{error && error.toString()}
						<br />
						{errorInfo && errorInfo.componentStack}
					</p>
				</div>
			);
		}

		return children;
	}
}

export default ErrorBoundary;
