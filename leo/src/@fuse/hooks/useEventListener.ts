import { useEffect, useRef } from 'react';

function useEventListener<T extends Event>(
	eventName: string,
	handler: (event: T) => void,
	element: HTMLElement | Window = window
) {
	// Create a mutable ref object to store the handler
	const savedHandler = useRef<(event: T) => void>(undefined);

	// Update ref.current value if handler changes
	useEffect(() => {
		savedHandler.current = handler;
	}, [handler]);

	useEffect(() => {
		// Check and ensure the element supports addEventListener
		const isSupported = Boolean(element && element.addEventListener);

		// Create event listener that calls handler function stored in ref
		const eventListener = (event: Event) => {
			if (savedHandler.current) {
				savedHandler.current(event as T);
			}
		};

		if (isSupported) {
			// Add event listener
			element.addEventListener(eventName, eventListener);
		}

		// Clean up event listener on component unmount
		return () => {
			if (isSupported) {
				element.removeEventListener(eventName, eventListener);
			}
		};
	}, [eventName, element]);
}

export default useEventListener;
