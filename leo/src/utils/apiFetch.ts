export const API_BASE_URL =
	process.env.NODE_ENV === 'development'
		? `http://localhost:${process.env.NEXT_PUBLIC_PORT || 3000}`
		: process.env.NEXT_PUBLIC_BASE_URL || '/';

// Define the types for options and configuration
type FetchOptions = RequestInit;

export class FetchApiError extends Error {
	status: number;

	data: unknown;

	constructor(status: number, data: unknown) {
		super(`FetchApiError: ${status}`);
		this.status = status;
		this.data = data;
	}
}

// Global headers configuration
export const globalHeaders: Record<string, string> = {};

// Function to update global headers
export const setGlobalHeaders = (newHeaders: Record<string, string>) => {
	Object.assign(globalHeaders, newHeaders);
};

export const removeGlobalHeaders = (headerKeys: string[]) => {
	headerKeys.forEach((key) => {
		delete globalHeaders[key];
	});
};

// Main apiFetch function with interceptors and type safety
const apiFetch = async (endpoint: string, options: FetchOptions = {}) => {
	const { headers, ...restOptions } = options;
	const method = restOptions.method || 'GET';
	// Set default headers, including global headers
	const config: FetchOptions = {
		headers: {
			...(method !== 'GET' && { 'Content-Type': 'application/json' }),
			...globalHeaders,
			...headers
		},
		...restOptions
	};

	try {
		const response = await fetch(`${API_BASE_URL}${endpoint}`, config);

		if (!response.ok) {
			throw new FetchApiError(response.status, await response.json());
		}

		return response;
	} catch (error) {
		console.error('Error in apiFetch:', error);
		throw error;
	}
};

export default apiFetch;
