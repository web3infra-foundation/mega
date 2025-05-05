'use client';

import Breadcrumbs, { BreadcrumbsProps } from '@mui/material/Breadcrumbs';
import { FuseNavItemType } from '@fuse/core/FuseNavigation/types/FuseNavItemType';
import usePathname from '@fuse/hooks/usePathname';

import Typography from '@mui/material/Typography';
import clsx from 'clsx';
import Link from '@fuse/core/Link';
import useNavigation from './theme-layouts/components/navigation/hooks/useNavigation';

type PageBreadcrumbProps = BreadcrumbsProps & {
	className?: string;
	skipHome?: boolean;
};

// Function to get the navigation item based on URL
function getNavigationItem(url: string, navigationItems: FuseNavItemType[]): FuseNavItemType {
	for (const item of navigationItems) {
		if (item.url === url) {
			return item;
		}

		if (item.children) {
			const childItem = getNavigationItem(url, item.children);

			if (childItem) {
				return childItem;
			}
		}
	}
	return null;
}

function PageBreadcrumb(props: PageBreadcrumbProps) {
	const { className, skipHome = false, ...rest } = props;
	const pathname = usePathname();
	const { navigation } = useNavigation();

	const crumbs = pathname
		.split('/')
		.filter(Boolean)
		.reduce(
			(acc: { title: string; url: string }[], part, index, array) => {
				const url = `/${array.slice(0, index + 1).join('/')}`;
				const navItem = getNavigationItem(url, navigation);
				const title = navItem?.title || part;

				acc.push({ title, url });
				return acc;
			},
			skipHome ? [] : [{ title: 'Home', url: '/' }]
		);

	return (
		<Breadcrumbs
			classes={{ ol: 'list-none m-0 p-0' }}
			className={clsx('flex w-full', className)}
			aria-label="breadcrumb"
			color="primary"
			{...rest}
		>
			{crumbs.map((item, index) => (
				<Typography
					component={item.url ? Link : 'span'}
					to={item.url}
					key={index}
					className="block font-medium tracking-tight capitalize max-w-32 truncate"
					role="button"
				>
					{item.title}
				</Typography>
			))}
		</Breadcrumbs>
	);
}

export default PageBreadcrumb;
