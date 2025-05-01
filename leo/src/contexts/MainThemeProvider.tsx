'use client';

import * as React from 'react';
import { useMemo } from 'react';
import rtlPlugin from 'stylis-plugin-rtl';
import FuseTheme from '@fuse/core/FuseTheme';
import { useMainTheme } from '@fuse/core/FuseSettings/hooks/fuseThemeHooks';
import createCache, { Options, StylisPlugin } from '@emotion/cache';
import { CacheProvider } from '@emotion/react';

type MainThemeProviderProps = {
	children: React.ReactNode;
};
const wrapInLayer: (layerName: string) => StylisPlugin = (layerName) => (node) => {
	if (node.root) {
		return;
	}

	// if we're at the root, replace node with `@layer layerName { node }`
	const child = { ...node, parent: node, root: node };
	Object.assign(node, {
		children: [child],
		length: 6,
		parent: null,
		props: [layerName],
		return: '',
		root: null,
		type: '@layer',
		value: `@layer ${layerName}`
	});
};

const emotionCacheOptions: Record<string, Options> = {
	rtl: {
		key: 'muirtl',
		stylisPlugins: [rtlPlugin, wrapInLayer('mui')],
		prepend: false
	},
	ltr: {
		key: 'muiltr',
		stylisPlugins: [wrapInLayer('mui')],
		prepend: false
	}
};

function MainThemeProvider({ children }: MainThemeProviderProps) {
	const mainTheme = useMainTheme();
	const langDirection = mainTheme?.direction;

	const cacheProviderValue = useMemo(() => createCache(emotionCacheOptions[langDirection]), [langDirection]);

	return (
		<CacheProvider value={cacheProviderValue}>
			<FuseTheme
				theme={mainTheme}
				root
			>
				{children}
			</FuseTheme>
		</CacheProvider>
	);
}

export default MainThemeProvider;
