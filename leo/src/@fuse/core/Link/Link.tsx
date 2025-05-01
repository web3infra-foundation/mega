'use client';

import NextLink, { LinkProps as NextLinkProps } from 'next/link';
import { ReactNode } from 'react';

type CustomLinkProps = Omit<NextLinkProps, 'href'> & {
	to?: string;
	href?: string;
	children?: ReactNode;
	className?: string;
	role?: string;
	ref?: React.RefObject<HTMLAnchorElement>;
	style?: React.CSSProperties;
	onKeyDown?: (event: React.KeyboardEvent<HTMLAnchorElement>) => void;
};

function Link(props: CustomLinkProps) {
	const { ref, to, href, children, className, role, ...rest } = props;

	return (
		<NextLink
			className={className}
			href={to || href || ''}
			role={role}
			ref={ref}
			{...rest}
		>
			{children}
		</NextLink>
	);
}

export default Link;
