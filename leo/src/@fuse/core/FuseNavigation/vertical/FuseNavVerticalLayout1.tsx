import List from '@mui/material/List';
import { styled } from '@mui/material/styles';
import clsx from 'clsx';
import FuseNavItem from '../FuseNavItem';
import { FuseNavigationProps } from '../FuseNavigation';
import { FuseNavItemType } from '../types/FuseNavItemType';

const StyledList = styled(List)(({ theme }) => ({
	'& .fuse-list-item': {
		'&:hover': {
			backgroundColor: 'rgba(0,0,0,.04)',
			...theme.applyStyles('dark', {
				backgroundColor: 'rgba(255, 255, 255, 0.05)'
			})
		},
		'&:focus:not(.active)': {
			backgroundColor: 'rgba(0,0,0,.05)',
			...theme.applyStyles('dark', {
				backgroundColor: 'rgba(255, 255, 255, 0.06)'
			})
		}
	},
	'& .fuse-list-item-text': {
		margin: 0
	},
	'& .fuse-list-item-text-primary': {
		lineHeight: '20px'
	},
	'&.active-square-list': {
		'& .fuse-list-item, & .active.fuse-list-item': {
			width: '100%',
			borderRadius: '0'
		}
	},
	'&.dense': {
		'& .fuse-list-item': {
			paddingTop: 0,
			paddingBottom: 0,
			height: 32
		}
	}
}));

/**
 * FuseNavVerticalLayout1
 * This component is used to render vertical navigations using
 * the Material-UI List component. It accepts the FuseNavigationProps props
 * and renders the FuseNavItem components accordingly
 */
function FuseNavVerticalLayout1(props: FuseNavigationProps) {
	const { navigation, active, dense, className, onItemClick, checkPermission } = props;

	function handleItemClick(item: FuseNavItemType) {
		onItemClick?.(item);
	}

	return (
		<StyledList
			className={clsx(
				'navigation whitespace-nowrap px-3 py-0',
				`active-${active}-list`,
				dense && 'dense',
				className
			)}
		>
			{navigation.map((_item) => (
				<FuseNavItem
					key={_item.id}
					type={`vertical-${_item.type}`}
					item={_item}
					nestedLevel={0}
					onItemClick={handleItemClick}
					checkPermission={checkPermission}
				/>
			))}
		</StyledList>
	);
}

export default FuseNavVerticalLayout1;
