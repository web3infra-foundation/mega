'use client';
import NavLinkAdapter from '@fuse/core/NavLinkAdapter';
import { styled } from '@mui/material/styles';
import Tooltip from '@mui/material/Tooltip';
import ListItemText from '@mui/material/ListItemText';
import clsx from 'clsx';
import { useMemo } from 'react';
import Box from '@mui/material/Box';
import { ListItemButton } from '@mui/material';
import Typography from '@mui/material/Typography';
import FuseNavBadge from '../../FuseNavBadge';
import FuseSvgIcon from '../../../FuseSvgIcon';
import { FuseNavigationProps } from '../../FuseNavigation';
import { FuseNavItemComponentProps } from '../../FuseNavItem';

const Root = styled(Box)(({ theme }) => ({
	'& > .fuse-list-item': {
		minHeight: 100,
		height: 100,
		width: 100,
		borderRadius: 12,
		margin: '0 0 4px 0',
		cursor: 'pointer',
		textDecoration: 'none!important',
		padding: 0,
		color: (theme) => `rgba(${theme.vars.palette.text.primaryChannel} / 0.7)`,
		'&.dense': {
			minHeight: 52,
			height: 52,
			width: 52
		},
		'&.type-divider': {
			padding: 0,
			height: 2,
			minHeight: 2,
			margin: '12px 0',
			backgroundColor: theme.vars.palette.divider,
			pointerEvents: 'none'
		},
		'&:hover': {
			color: theme.vars.palette.text.primary
		},
		'&.active': {
			color: theme.vars.palette.text.primary,
			backgroundColor: 'rgba(255, 255, 255, .1)!important',
			transition: 'border-radius .15s cubic-bezier(0.4,0.0,0.2,1)',
			'& .fuse-list-item-text-primary': {
				color: 'inherit'
			},
			'& .fuse-list-item-icon': {
				color: 'inherit'
			},
			...theme.applyStyles('light', {
				backgroundColor: 'rgba(0, 0, 0, .05)!important'
			})
		},
		'& .fuse-list-item-icon': {
			color: 'inherit'
		},
		'& .fuse-list-item-text': {}
	}
}));

export type FuseNavVerticalTabProps = Omit<FuseNavigationProps, 'navigation'> & FuseNavItemComponentProps;

/**
 *  The `FuseNavVerticalTab` component renders vertical navigation item with an adaptable
 *  layout to be used within the `FuseNavigation`. It only supports the `type`s of 'item',
 *  'selection' and 'divider'
 * */
function FuseNavVerticalTab(props: FuseNavVerticalTabProps) {
	const { item, onItemClick, firstLevel, dense, selectedId, checkPermission } = props;
	const component = item.url ? NavLinkAdapter : 'li';

	const itemProps = useMemo(
		() => ({
			...(component !== 'li' && {
				disabled: item.disabled,
				to: item.url,
				end: item.end,
				role: 'button'
			})
		}),
		[item, component]
	);

	const memoizedContent = useMemo(
		() => (
			<Root sx={item.sx}>
				<ListItemButton
					component={component}
					className={clsx(
						`type-${item.type}`,
						dense && 'dense',
						selectedId === item.id && 'active',
						'fuse-list-item flex flex-col items-center justify-center p-3'
					)}
					onClick={() => onItemClick && onItemClick(item)}
					{...itemProps}
				>
					{dense ? (
						<Tooltip
							title={item.title || ''}
							placement="right"
						>
							<div className="relative flex h-8 min-h-8 w-8 items-center justify-center">
								{item.icon ? (
									<FuseSvgIcon
										className={clsx('fuse-list-item-icon', item.iconClass)}
										color="action"
									>
										{item.icon}
									</FuseSvgIcon>
								) : (
									item.title && <Typography className="text-lg font-bold">{item.title[0]}</Typography>
								)}
								{item.badge && (
									<FuseNavBadge
										badge={item.badge}
										className="absolute top-0 h-4 min-w-4 justify-center p-1 ltr:right-0 rtl:left-0"
									/>
								)}
							</div>
						</Tooltip>
					) : (
						<>
							<div className="relative mb-2 flex h-8 min-h-8 w-8 items-center justify-center">
								{item.icon ? (
									<FuseSvgIcon
										size={32}
										className={clsx('fuse-list-item-icon', item.iconClass)}
										color="action"
									>
										{item.icon}
									</FuseSvgIcon>
								) : (
									item.title && (
										<Typography className="text-2xl font-bold">{item.title[0]}</Typography>
									)
								)}
								{item.badge && (
									<FuseNavBadge
										badge={item.badge}
										className="absolute top-0 h-4 min-w-4 justify-center p-1 ltr:right-0 rtl:left-0"
									/>
								)}
							</div>

							<ListItemText
								className="fuse-list-item-text w-full grow-0 px-2"
								primary={item.title}
								classes={{
									primary:
										'text-md font-medium fuse-list-item-text-primary truncate text-center truncate'
								}}
							/>
						</>
					)}
				</ListItemButton>
				{!firstLevel &&
					item.children &&
					item.children.map((_item) => (
						<NavVerticalTab
							key={_item.id}
							type={`vertical-${_item.type}`}
							item={_item}
							nestedLevel={0}
							onItemClick={onItemClick}
							dense={dense}
							selectedId={selectedId}
							checkPermission={checkPermission}
						/>
					))}
			</Root>
		),
		[item, component, dense, selectedId, itemProps, firstLevel, onItemClick, checkPermission]
	);

	if (checkPermission && !item?.hasPermission) {
		return null;
	}

	return memoizedContent;
}

const NavVerticalTab = FuseNavVerticalTab;

export default NavVerticalTab;
