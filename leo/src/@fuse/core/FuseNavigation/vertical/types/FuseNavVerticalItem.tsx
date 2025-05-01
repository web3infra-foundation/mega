'use client';
import NavLinkAdapter from '@fuse/core/NavLinkAdapter';
import { styled } from '@mui/material/styles';
import ListItemText from '@mui/material/ListItemText';
import clsx from 'clsx';
import { useMemo } from 'react';
import { ListItemButton, ListItemButtonProps } from '@mui/material';
import FuseNavBadge from '../../FuseNavBadge';
import FuseSvgIcon from '../../../FuseSvgIcon';
import { FuseNavItemComponentProps } from '../../FuseNavItem';

type ListItemButtonStyleProps = ListItemButtonProps & {
	itempadding: number;
};

const Root = styled(ListItemButton)<ListItemButtonStyleProps>(({ theme, ...props }) => ({
	minHeight: 36,
	width: '100%',
	borderRadius: '8px',
	margin: '0 0 4px 0',
	paddingRight: 16,
	paddingLeft: props.itempadding > 80 ? 80 : props.itempadding,
	paddingTop: 10,
	paddingBottom: 10,
	color: `rgba(${theme.vars.palette.text.primaryChannel} / 0.7)`,
	cursor: 'pointer',
	textDecoration: 'none!important',
	'&:hover': {
		color: theme.vars.palette.text.primary
	},
	'&.active': {
		color: theme.vars.palette.text.primary,
		backgroundColor:
			theme.palette.mode === 'light' ? 'rgba(0, 0, 0, .05)!important' : 'rgba(255, 255, 255, .1)!important',
		transition: 'border-radius .15s cubic-bezier(0.4,0.0,0.2,1)',
		'& > .fuse-list-item-text-primary': {
			color: 'inherit'
		},
		'& > .fuse-list-item-icon': {
			color: 'inherit'
		}
	},
	'& > .fuse-list-item-icon': {
		marginRight: 16,
		color: 'inherit'
	},
	'& > .fuse-list-item-text': {}
}));

/**
 * FuseNavVerticalItem is a React component used to render FuseNavItem as part of the Fuse navigational component.
 */
function FuseNavVerticalItem(props: FuseNavItemComponentProps) {
	const { item, nestedLevel = 0, onItemClick, checkPermission } = props;
	const itempadding = nestedLevel > 0 ? 38 + nestedLevel * 16 : 16;
	const component = item.url ? NavLinkAdapter : 'li';

	const itemProps = useMemo(
		() => ({
			...(component !== 'li' && {
				disabled: item.disabled,
				to: item.url || '',
				end: item.end,
				role: 'button',
				exact: item?.exact
			})
		}),
		[item, component]
	);

	const memoizedContent = useMemo(
		() => (
			<Root
				component={component}
				className={clsx('fuse-list-item', item.active && 'active')}
				onClick={() => onItemClick && onItemClick(item)}
				itempadding={itempadding}
				sx={item.sx}
				{...itemProps}
			>
				{item.icon && (
					<FuseSvgIcon
						className={clsx('fuse-list-item-icon shrink-0', item.iconClass)}
						color="action"
					>
						{item.icon}
					</FuseSvgIcon>
				)}

				<ListItemText
					className="fuse-list-item-text"
					primary={item.title}
					secondary={item.subtitle}
					classes={{
						primary: 'text-md font-medium fuse-list-item-text-primary truncate',
						secondary: 'text-sm font-medium fuse-list-item-text-secondary leading-[1.5] truncate'
					}}
				/>
				{item.badge && <FuseNavBadge badge={item.badge} />}
			</Root>
		),
		[component, item, itemProps, itempadding, onItemClick]
	);

	if (checkPermission && !item?.hasPermission) {
		return null;
	}

	return memoizedContent;
}

const NavVerticalItem = FuseNavVerticalItem;

export default NavVerticalItem;
