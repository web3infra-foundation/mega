import { styled } from '@mui/material/styles';
import clsx from 'clsx';
import { memo } from 'react';
import { FuseNavBadgeType } from './types/FuseNavBadgeType';

const Root = styled('div')(({ theme }) => ({
	padding: '0 7px',
	fontSize: 11,
	fontWeight: 600,
	height: 20,
	minWidth: 20,
	borderRadius: 20,
	display: 'flex',
	alignItems: 'center',
	backgroundColor: theme.vars.palette.secondary.main,
	color: theme.vars.palette.secondary.contrastText
}));

type FuseNavBadgeProps = {
	className?: string;
	classes?: string;
	badge: FuseNavBadgeType;
};

/**
 * FuseNavBadge component.
 * This component will render a badge on a FuseNav element. It accepts a `FuseNavBadgeType` as a prop,
 * which is an object containing a title and background and foreground colour.
 */
function FuseNavBadge(props: FuseNavBadgeProps) {
	const { className = '', classes = '', badge } = props;

	return (
		<Root
			className={clsx('item-badge', className, classes)}
			style={{
				backgroundColor: badge.bg,
				color: badge.fg
			}}
		>
			{badge.title}
		</Root>
	);
}

export default memo(FuseNavBadge);
