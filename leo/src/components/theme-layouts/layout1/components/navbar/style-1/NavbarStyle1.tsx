import { styled, Theme } from '@mui/material/styles';
import SwipeableDrawer from '@mui/material/SwipeableDrawer';
import { useAppDispatch, useAppSelector } from 'src/store/hooks';
import {
	navbarCloseMobile,
	resetNavbar,
	selectFuseNavbar
} from 'src/components/theme-layouts/components/navbar/navbarSlice';
import { useEffect } from 'react';
import useFuseLayoutSettings from '@fuse/core/FuseLayout/useFuseLayoutSettings';
import useThemeMediaQuery from '@fuse/hooks/useThemeMediaQuery';
import NavbarStyle1Content from './NavbarStyle1Content';
import { Layout1ConfigDefaultsType } from '@/components/theme-layouts/layout1/Layout1Config';

const navbarWidth = 280;

type StyledNavBarProps = {
	theme?: Theme;
	open: boolean;
	position: string;
};

const StyledNavBar = styled('div')<StyledNavBarProps>(({ theme }) => ({
	minWidth: navbarWidth,
	width: navbarWidth,
	maxWidth: navbarWidth,
	variants: [
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
				marginLeft: `-${navbarWidth}px`
			}
		},
		{
			props: ({ open, position }) => !open && position === 'right',
			style: {
				marginRight: `-${navbarWidth}px`
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
		},
		{
			props: ({ open, position }) => open && position === 'left',
			style: {
				borderRight: `1px solid ${theme.vars.palette.divider}`
			}
		},
		{
			props: ({ open, position }) => open && position === 'right',
			style: {
				borderLeft: `1px solid ${theme.vars.palette.divider}`
			}
		}
	]
}));

const StyledNavBarMobile = styled(SwipeableDrawer)(() => ({
	'& .MuiDrawer-paper': {
		minWidth: navbarWidth,
		width: navbarWidth,
		maxWidth: navbarWidth
	}
}));

/**
 * The navbar style 1.
 */
function NavbarStyle1() {
	const dispatch = useAppDispatch();
	const isMobile = useThemeMediaQuery((theme) => theme.breakpoints.down('lg'));

	const settings = useFuseLayoutSettings();
	const config = settings.config as Layout1ConfigDefaultsType;

	const navbar = useAppSelector(selectFuseNavbar);

	useEffect(() => {
		return () => {
			dispatch(resetNavbar());
		};
	}, [dispatch]);

	return (
		<>
			{!isMobile && (
				<StyledNavBar
					className="sticky top-0 z-20 h-screen flex-auto shrink-0 flex-col overflow-hidden shadow-sm"
					open={navbar.open}
					position={config.navbar.position}
				>
					<NavbarStyle1Content />
				</StyledNavBar>
			)}

			{isMobile && (
				<StyledNavBarMobile
					classes={{
						paper: 'flex-col flex-auto h-full'
					}}
					anchor={config.navbar.position as 'left' | 'top' | 'right' | 'bottom'}
					variant="temporary"
					open={navbar.mobileOpen}
					onClose={() => dispatch(navbarCloseMobile())}
					onOpen={() => {}}
					disableSwipeToOpen
					ModalProps={{
						keepMounted: true // Better open performance on mobile.
					}}
				>
					<NavbarStyle1Content />
				</StyledNavBarMobile>
			)}
		</>
	);
}

export default NavbarStyle1;
