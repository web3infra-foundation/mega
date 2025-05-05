'use client';

import { useEffect, useState } from 'react';
import { useTheme, Theme } from '@mui/material/styles';

/**
 * The useThemeMediaQuery function is a custom hook that returns a boolean indicating whether the current screen matches the specified media query.
 * It takes in a themeCallbackFunc as a parameter, which is a function that returns a string representing the media query to match.
 * It returns a boolean indicating whether the current screen matches the specified media query.
 */
function useThemeMediaQuery(themeCallbackFunc: (theme: Theme) => string) {
	const theme = useTheme();

	const query = themeCallbackFunc(theme).replace('@media ', '');

	// State to track whether the component has mounted
	const [hasMounted, setHasMounted] = useState(false);
	const [matches, setMatches] = useState(false);

	useEffect(() => {
		// After mounting, we can safely access the `window` object and evaluate the media query
		setHasMounted(true);
	}, []);

	useEffect(() => {
		if (hasMounted) {
			const mediaQuery = window.matchMedia(query);

			// Update the state with the current value
			setMatches(mediaQuery.matches);

			// Create an event listener to update state when the query matches change
			const handler = (event: MediaQueryListEvent) => setMatches(event.matches);
			mediaQuery.addEventListener('change', handler);

			// Cleanup event listener on unmount
			return () => mediaQuery.removeEventListener('change', handler);
		}

		return undefined;
	}, [query, hasMounted]);

	// Prevent rendering mismatched content by ensuring consistent SSR and client behavior
	if (!hasMounted) {
		return false; // Prevents server-client mismatch by avoiding media queries until client-side render
	}

	return matches;
}

export default useThemeMediaQuery;
