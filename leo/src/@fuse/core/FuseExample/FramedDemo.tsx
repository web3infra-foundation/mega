import * as React from 'react';
import { useTheme } from '@mui/material/styles';
import createCache from '@emotion/cache';
import rtlPlugin from 'stylis-plugin-rtl';
import { CacheProvider } from '@emotion/react';
import GlobalStyles from '@mui/material/GlobalStyles';
import { StyleSheetManager } from 'styled-components';
import { ReactElement } from 'react';

type FramedDemoProps = {
	document: Document;
	children: ReactElement<{ window?: () => Window }>;
};

/**
 * Renders document wrapped with emotion and styling-components cache providers, and proper direction for rtl theme.
 * This also add window property to the child with `getWindow` function, which is useful to fetch window property.
 */
function FramedDemo(props: FramedDemoProps) {
	const { children, document } = props;

	const theme = useTheme();
	React.useEffect(() => {
		document.body.dir = theme.direction;
	}, [document, theme.direction]);

	const cache = React.useMemo(
		() =>
			createCache({
				key: `iframe-demo-${theme.direction}`,
				prepend: true,
				container: document.head,
				stylisPlugins: theme.direction === 'rtl' ? [rtlPlugin] : []
			}),
		[document, theme.direction]
	);

	const getWindow = React.useCallback(() => document.defaultView, [document]);

	return (
		<StyleSheetManager
			target={document.head}
			stylisPlugins={theme.direction === 'rtl' ? [rtlPlugin] : []}
		>
			<CacheProvider value={cache}>
				<GlobalStyles
					styles={() => ({
						html: {
							fontSize: '62.5%'
						}
					})}
				/>
				{React.cloneElement(children, {
					window: getWindow
				})}
			</CacheProvider>
		</StyleSheetManager>
	);
}

export default FramedDemo;
