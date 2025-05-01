import { styled } from '@mui/material/styles';
import SwipeableDrawer from '@mui/material/SwipeableDrawer';
import {
	navbarCloseFolded,
	navbarCloseMobile,
	navbarOpenFolded,
	resetNavbar,
	selectFuseNavbar
} from 'src/components/theme-layouts/components/navbar/navbarSlice';
import { useAppDispatch, useAppSelector } from 'src/store/hooks';

import { Theme } from '@mui/system';
import { useEffect } from 'react';
import useFuseLayoutSettings from '@fuse/core/FuseLayout/useFuseLayoutSettings';
import useThemeMediaQuery from '@fuse/hooks/useThemeMediaQuery';
import NavbarStyle2Content from './NavbarStyle2Content';
import { Layout1ConfigDefaultsType } from '@/components/theme-layouts/layout1/Layout1Config';

const navbarWidth = 280;

type StyledNavBarPropsProps = {
	theme?: Theme;
	folded: number;
	open: boolean;
};

const Root = styled('div')<StyledNavBarPropsProps>(({ theme }) => ({
	display: 'flex',
	flexDirection: 'column',
	zIndex: 4,
	[theme.breakpoints.up('lg')]: {
		width: navbarWidth,
		minWidth: navbarWidth
	},
	variants: [
		{
			props: ({ folded }) => folded,
			style: {
				[theme.breakpoints.up('lg')]: {
					width: 76,
					minWidth: 76
				}
			}
		}
	]
}));

type StyledNavBarProps = {
	theme?: Theme;
	open?: boolean;
	folded: number;
	foldedandopened: number;
	foldedandclosed: number;
	position?: string;
	anchor?: string;
};

const StyledNavbar = styled('div')<StyledNavBarProps>(({ theme }) => ({
	minWidth: navbarWidth,
	width: navbarWidth,
	maxWidth: navbarWidth,
	maxHeight: '100%',
	transition: theme.transitions.create(['width', 'min-width'], {
		easing: theme.transitions.easing.sharp,
		duration: theme.transitions.duration.shorter
	}),
	variants: [
		{
			props: {
				position: 'left'
			},
			style: {
				borderRight: `1px solid ${theme.vars.palette.divider}`,
				left: 0
			}
		},
		{
			props: {
				position: 'right'
			},
			style: {
				borderLight: `1px solid ${theme.vars.palette.divider}`,
				right: 0
			}
		},
		{
			props: ({ folded }) => folded,
			style: {
				position: 'absolute',
				width: 76,
				minWidth: 76,
				top: 0,
				bottom: 0
			}
		},
		{
			props: ({ foldedandopened }) => foldedandopened,
			style: {
				width: navbarWidth,
				minWidth: navbarWidth
			}
		},
		{
			props: ({ foldedandclosed }) => foldedandclosed,
			style: {
				'& .NavbarStyle2-content': {
					'& .logo-icon': {
						width: 44,
						height: 44
					},
					'& .logo-text': {
						opacity: 0
					},
					'& .react-badge': {
						opacity: 0
					},
					'& .fuse-list-item': {
						width: 52
					},
					'& .fuse-list-item-text, & .arrow-icon, & .item-badge': {
						opacity: 0
					},
					'& .fuse-list-subheader .fuse-list-subheader-text': {
						opacity: 0
					},
					'& .fuse-list-subheader:before': {
						content: '""',
						display: 'block',
						position: 'absolute',
						minWidth: 16,
						borderTop: '2px solid',
						opacity: 0.2
					},
					'& .collapse-children': {
						display: 'none'
					},
					'& .user-menu': {
						minWidth: 52,
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
				}
			}
		}
	]
}));

const StyledNavbarMobile = styled(SwipeableDrawer)<StyledNavBarProps>(({ theme }) => ({
	'& > .MuiDrawer-paper': {
		minWidth: navbarWidth,
		width: navbarWidth,
		maxWidth: navbarWidth,
		maxHeight: '100%',
		transition: theme.transitions.create(['width', 'min-width'], {
			easing: theme.transitions.easing.sharp,
			duration: theme.transitions.duration.shorter
		})
	}
}));

/**
 * The navbar style 2.
 */
function NavbarStyle2() {
	const dispatch = useAppDispatch();

	const settings = useFuseLayoutSettings();
	const config = settings.config as Layout1ConfigDefaultsType;
	const isMobile = useThemeMediaQuery((theme) => theme.breakpoints.down('lg'));

	const navbar = useAppSelector(selectFuseNavbar);

	const folded = config.navbar?.folded;
	const foldedandclosed = folded && !navbar.foldedOpen;
	const foldedandopened = folded && navbar.foldedOpen;

	useEffect(() => {
		return () => {
			dispatch(resetNavbar());
		};
	}, [dispatch]);

	return (
		<Root
			folded={folded ? 1 : 0}
			open={navbar.open}
			id="fuse-navbar"
			className="sticky top-0 z-20 h-screen shrink-0"
		>
			{!isMobile && (
				<StyledNavbar
					className="hidden lg:flex sticky top-0 z-20 h-screen flex-auto shrink-0 flex-col overflow-hidden shadow-sm"
					position={config?.navbar?.position}
					folded={folded ? 1 : 0}
					foldedandopened={foldedandopened ? 1 : 0}
					foldedandclosed={foldedandclosed ? 1 : 0}
					onMouseEnter={() => foldedandclosed && dispatch(navbarOpenFolded())}
					onMouseLeave={() => foldedandopened && dispatch(navbarCloseFolded())}
				>
					<NavbarStyle2Content className="NavbarStyle2-content" />
				</StyledNavbar>
			)}

			{isMobile && (
				<StyledNavbarMobile
					classes={{
						root: 'flex lg:hidden',
						paper: 'flex-col flex-auto h-full'
					}}
					folded={folded ? 1 : 0}
					foldedandopened={foldedandopened ? 1 : 0}
					foldedandclosed={foldedandclosed ? 1 : 0}
					anchor={config?.navbar?.position as 'left' | 'top' | 'right' | 'bottom'}
					variant="temporary"
					open={navbar.mobileOpen}
					onClose={() => dispatch(navbarCloseMobile())}
					onOpen={() => {}}
					disableSwipeToOpen
					ModalProps={{
						keepMounted: true // Better open performance on mobile.
					}}
				>
					<NavbarStyle2Content className="NavbarStyle2-content" />
				</StyledNavbarMobile>
			)}
		</Root>
	);
}

export default NavbarStyle2;
