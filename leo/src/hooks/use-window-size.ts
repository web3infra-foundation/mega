import * as React from 'react';

export function useWindowSize() {
	const [windowSize, setWindowSize] = React.useState({
		width: 0,
		height: 0,
		offsetTop: 0
	});

	React.useEffect(() => {
		const handleResize = () => {
			if (typeof window !== 'undefined') {
				const width = window.visualViewport?.width || 0;
				const height = window.visualViewport?.height || 0;
				const offsetTop = window.visualViewport?.offsetTop || 0;

				setWindowSize((state) => {
					if (width === state.width && height === state.height && offsetTop === state.offsetTop) {
						return state;
					}

					return { width, height, offsetTop };
				});
			}
		};

		window.visualViewport?.addEventListener('resize', handleResize);
		window.visualViewport?.addEventListener('scroll', handleResize);

		return () => {
			window.visualViewport?.removeEventListener('resize', handleResize);
			window.visualViewport?.removeEventListener('scroll', handleResize);
		};
	}, []);

	return windowSize;
}
