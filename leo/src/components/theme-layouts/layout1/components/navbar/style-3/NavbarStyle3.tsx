import { styled } from '@mui/material/styles';
import SwipeableDrawer from '@mui/material/SwipeableDrawer';
import { useAppDispatch, useAppSelector } from 'src/store/hooks';
import {
	navbarCloseMobile,
	resetNavbar,
	selectFuseNavbar
} from 'src/components/theme-layouts/components/navbar/navbarSlice';
import GlobalStyles from '@mui/material/GlobalStyles';
import { Theme } from '@mui/system';
import clsx from 'clsx';
import { useEffect } from 'react';
import useFuseLayoutSettings from '@fuse/core/FuseLayout/useFuseLayoutSettings';
import useThemeMediaQuery from '@fuse/hooks/useThemeMediaQuery';
import NavbarStyle3Content from './NavbarStyle3Content';
import { Layout1ConfigDefaultsType } from '@/components/theme-layouts/layout1/Layout1Config';

const navbarWidth = 120;
const navbarWidthDense = 64;
const panelWidth = 280;

type StyledNavBarProps = {
	theme?: Theme;
	open?: boolean;
	folded?: number;
	dense?: number;
	position?: string;
	className?: string;
	anchor?: string;
};

const StyledNavBar = styled('div')<StyledNavBarProps>(({ theme, dense }) => ({
	minWidth: navbarWidth,
	width: navbarWidth,
	maxWidth: navbarWidth,
	'& .user-menu': {
		minWidth: 56,
		width: 56,
		'& .title': {
			opacity: 0
		},
		'& .subtitle': {
			opacity: 0
		},
		'& .info-icon': {
			opacity: 0
		},
		'& .arrow': {
			opacity: 0
		}
	},
	variants: [
		{
			props: {
				position: 'left'
			},
			style: {
				borderRight: `1px solid ${theme.vars.palette.divider}`
			}
		},
		{
			props: {
				position: 'right'
			},
			style: {
				borderLight: `1px solid ${theme.vars.palette.divider}`
			}
		},
		{
			props: ({ dense }) => dense,
			style: {
				minWidth: navbarWidthDense,
				width: navbarWidthDense,
				maxWidth: navbarWidthDense
			}
		},
		{
			props: ({ dense, open, position }) => dense && !open && position === 'left',
			style: {
				marginLeft: -navbarWidthDense
			}
		},
		{
			props: ({ dense, open, position }) => dense && !open && position === 'right',
			style: {
				marginRight: -navbarWidthDense
			}
		},
		{
			props: ({ folded }) => !folded,
			style: {
				minWidth: navbarWidth + panelWidth,
				width: navbarWidth + panelWidth,
				maxWidth: navbarWidth + panelWidth,
				'& #fuse-navbar-panel': {
					opacity: '1!important',
					pointerEvents: 'initial!important'
				}
			}
		},
		{
			props: ({ folded, dense }) => !folded && dense,
			style: {
				minWidth: navbarWidthDense + panelWidth
			}
		},
		{
			props: ({ folded, dense }) => !folded && dense,
			style: {
				width: navbarWidthDense + panelWidth
			}
		},
		{
			props: ({ folded, dense }) => !folded && dense,
			style: {
				maxWidth: navbarWidthDense + panelWidth
			}
		},
		{
			props: ({ folded, open, position }) => !folded && !open && position === 'left',
			style: {
				marginLeft: `${-(dense ? navbarWidthDense + panelWidth : navbarWidth + panelWidth)}px!important`
			}
		},
		{
			props: ({ folded, open, position }) => !folded && !open && position === 'right',
			style: {
				marginRight: `${-(dense ? navbarWidthDense + panelWidth : navbarWidth + panelWidth)}px!important`
			}
		},
		{
			props: ({ open }) => !open,
			style: {
				transition: theme.transitions.create('margin', {
					easing: theme.transitions.easing.easeOut,
					duration: theme.transitions.duration.leavingScreen
				})
			}
		},
		{
			props: ({ open, position }) => !open && position === 'left',
			style: {
				marginLeft: -(dense ? navbarWidthDense : navbarWidth)
			}
		},
		{
			props: ({ open, position }) => !open && position === 'right',
			style: {
				marginRight: -(dense ? navbarWidthDense : navbarWidth)
			}
		},
		{
			props: ({ open }) => open,
			style: {
				transition: theme.transitions.create('margin', {
					easing: theme.transitions.easing.easeOut,
					duration: theme.transitions.duration.enteringScreen
				})
			}
		}
	]
}));

const StyledNavBarMobile = styled(SwipeableDrawer)<StyledNavBarProps>(() => ({
	'& .MuiDrawer-paper': {
		'& #fuse-navbar-side-panel': {
			minWidth: 'auto',
			wdith: 'auto'
		},
		'& #fuse-navbar-panel': {
			opacity: '1!important',
			pointerEvents: 'initial!important'
		}
	},
	'& .user-menu': {
		minWidth: 56,
		width: 56,
		'& .title': {
			opacity: 0
		},
		'& .subtitle': {
			opacity: 0
		},
		'& .info-icon': {
			opacity: 0
		},
		'& .arrow': {
			opacity: 0
		}
	}
}));

type NavbarStyle3Props = {
	className?: string;
	dense?: boolean;
};

/**
 * The navbar style 3.
 */
function NavbarStyle3(props: NavbarStyle3Props) {
	const { className = '', dense = false } = props;
	const dispatch = useAppDispatch();

	const settings = useFuseLayoutSettings();
	const config = settings.config as Layout1ConfigDefaultsType;
	const isMobile = useThemeMediaQuery((theme) => theme.breakpoints.down('lg'));

	const { folded } = config.navbar;

	const navbar = useAppSelector(selectFuseNavbar);

	useEffect(() => {
		return () => {
			dispatch(resetNavbar());
		};
	}, [dispatch]);

	return (
		<>
			<GlobalStyles
				styles={(theme) => ({
					'& #fuse-navbar-side-panel': {
						width: dense ? navbarWidthDense : navbarWidth,
						minWidth: dense ? navbarWidthDense : navbarWidth,
						maxWidth: dense ? navbarWidthDense : navbarWidth
					},
					'& #fuse-navbar-panel': {
						maxWidth: '100%',
						width: panelWidth,
						borderRight: `1px solid ${theme.vars.palette.divider}!important`,
						borderLeft: `1px solid ${theme.vars.palette.divider}!important`,
						[theme.breakpoints.up('lg')]: {
							minWidth: panelWidth,
							maxWidth: 'initial'
						}
					}
				})}
			/>

			{!isMobile && (
				<StyledNavBar
					open={navbar.open}
					dense={dense ? 1 : 0}
					folded={folded ? 1 : 0}
					position={config.navbar.position}
					className={clsx('sticky top-0 z-20 h-screen flex-auto shrink-0 flex-col', className)}
				>
					<NavbarStyle3Content dense={dense ? 1 : 0} />
				</StyledNavBar>
			)}

			{isMobile && (
				<StyledNavBarMobile
					classes={{
						paper: clsx('h-screen w-auto max-w-full flex-auto flex-col overflow-hidden', className)
					}}
					anchor={config.navbar.position as 'left' | 'right'}
					variant="temporary"
					open={navbar.mobileOpen}
					onClose={() => dispatch(navbarCloseMobile())}
					onOpen={() => {}}
					disableSwipeToOpen
					ModalProps={{
						keepMounted: true // Better open performance on mobile.
					}}
				>
					<NavbarStyle3Content dense={dense ? 1 : 0} />
				</StyledNavBarMobile>
			)}
		</>
	);
}

export default NavbarStyle3;
