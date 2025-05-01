import Fab from '@mui/material/Fab';
import { styled } from '@mui/material/styles';

import Tooltip from '@mui/material/Tooltip';
import clsx from 'clsx';
import FuseSvgIcon from '@fuse/core/FuseSvgIcon';

const Root = styled(Tooltip)<{ position: 'left' | 'right' }>(({ theme }) => ({
	'& > .button': {
		height: 40,
		position: 'absolute',
		zIndex: 99,
		top: 12,
		width: 24,
		borderRadius: 38,
		padding: 8,
		backgroundColor: theme.vars.palette.background.paper,
		transition: theme.transitions.create(['background-color', 'border-radius', 'width', 'min-width', 'padding'], {
			easing: theme.transitions.easing.easeInOut,
			duration: theme.transitions.duration.shorter
		}),
		'&:hover': {
			width: 52,
			paddingLeft: 8,
			paddingRight: 8
		},
		'& > .button-icon': {
			fontSize: 18,
			transition: theme.transitions.create(['transform'], {
				easing: theme.transitions.easing.easeInOut,
				duration: theme.transitions.duration.short
			})
		}
	},
	variants: [
		{
			props: {
				position: 'left'
			},
			style: {
				'& > .button': {
					borderBottomLeftRadius: 0,
					borderTopLeftRadius: 0,
					paddingLeft: 4,
					left: 0
				}
			}
		},
		{
			props: {
				position: 'right'
			},
			style: {
				'& > .button': {
					borderBottomRightRadius: 0,
					borderTopRightRadius: 0,
					paddingRight: 4,
					right: 0,
					'& > .button-icon': {
						transform: 'rotate(-180deg)'
					}
				}
			}
		}
	]
}));

type NavbarToggleFabProps = {
	className?: string;
	position?: string;
	onClick?: () => void;
};

/**
 * The NavbarToggleFab component.
 */
function NavbarToggleFab(props: NavbarToggleFabProps) {
	const { className = '', position = 'left', onClick } = props;

	return (
		<Root
			title="Show Navigation"
			placement={position === 'left' ? 'right' : 'left'}
			position={position as 'left' | 'right'}
		>
			<Fab
				className={clsx('button', className)}
				onClick={onClick}
				disableRipple
			>
				<FuseSvgIcon
					color="action"
					className="button-icon"
				>
					heroicons-outline:bars-3
				</FuseSvgIcon>
			</Fab>
		</Root>
	);
}

export default NavbarToggleFab;
