import type { Metadata } from 'next';

async function generateMetadata(meta: {
	title: string;
	description: string;
	cardImage: string;
	robots: string;
	favicon: string;
	url: string;
}): Promise<Metadata> {
	return {
		title: meta.title,
		description: meta.description,
		referrer: 'origin-when-cross-origin',
		keywords: ['git', 'buck2', 'gitmega', 'monorepo'],
		authors: [{ name: 'gitmega', url: 'https://gitmega.dev' }],
		creator: 'gitmega.dev',
		publisher: 'gitmega.dev',
		robots: meta.robots,
		icons: { icon: meta.favicon },
		metadataBase: new URL(meta.url),
		openGraph: {
			url: meta.url,
			title: meta.title,
			description: meta.description,
			images: [meta.cardImage],
			type: 'website',
			siteName: meta.title
		},
		twitter: {
			card: 'summary_large_image',
			site: '@genedna',
			creator: '@genedna',
			title: meta.title,
			description: meta.description,
			images: [meta.cardImage]
		}
	};
}

export default generateMetadata;
