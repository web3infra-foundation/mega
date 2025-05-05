import { styled, ThemeProvider } from '@mui/material/styles';
import SwipeableDrawer from '@mui/material/SwipeableDrawer';
import { memo, useEffect } from 'react';
import { useAppDispatch, useAppSelector } from 'src/store/hooks';
import useThemeMediaQuery from '@fuse/hooks/useThemeMediaQuery';
import {
	navbarCloseMobile,
	navbarSlice,
	selectFuseNavbar
} from 'src/components/theme-layouts/components/navbar/navbarSlice';
import NavbarToggleFab from 'src/components/theme-layouts/components/navbar/NavbarToggleFab';
import withSlices from 'src/store/withSlices';
import usePathname from '@fuse/hooks/usePathname';
import { useNavbarTheme } from '@fuse/core/FuseSettings/hooks/fuseThemeHooks';
import useFuseLayoutSettings from '@fuse/core/FuseLayout/useFuseLayoutSettings';
import NavbarLayout3 from './NavbarLayout3';
import NavbarMobileLayout3 from './NavbarMobileLayout3';

const StyledSwipeableDrawer = styled(SwipeableDrawer)(({ theme }) => ({
	'& > .MuiDrawer-paper': {
		height: '100%',
		flexDirection: 'column',
		flex: '1 1 auto',
		width: 280,
		minWidth: 280,
		transition: theme.transitions.create(['width', 'min-width'], {
			easing: theme.transitions.easing.sharp,
			duration: theme.transitions.duration.shorter
		})
	}
}));

type NavbarWrapperLayout3Props = {
	className?: string;
};

/**
 * The navbar wrapper layout 3.
 */
function NavbarWrapperLayout3(props: NavbarWrapperLayout3Props) {
	const { className = '' } = props;

	const dispatch = useAppDispatch();
	const { config } = useFuseLayoutSettings();
	const navbarTheme = useNavbarTheme();
	const navbar = useAppSelector(selectFuseNavbar);
	const pathname = usePathname();
	const isMobile = useThemeMediaQuery((theme) => theme.breakpoints.down('lg'));

	useEffect(() => {
		if (isMobile) {
			dispatch(navbarCloseMobile());
		}
	}, [pathname, isMobile, dispatch]);

	return (
		<>
			<ThemeProvider theme={navbarTheme}>
				{!isMobile && <NavbarLayout3 className={className} />}

				{isMobile && (
					<StyledSwipeableDrawer
						anchor="left"
						variant="temporary"
						open={navbar.mobileOpen}
						onClose={() => dispatch(navbarCloseMobile())}
						onOpen={() => {}}
						disableSwipeToOpen
						ModalProps={{
							keepMounted: true // Better open performance on mobile.
						}}
					>
						<NavbarMobileLayout3 />
					</StyledSwipeableDrawer>
				)}
			</ThemeProvider>

			{config.navbar.display && !config.toolbar.display && isMobile && <NavbarToggleFab />}
		</>
	);
}

const NavbarWithSlices = withSlices<NavbarWrapperLayout3Props>([navbarSlice])(memo(NavbarWrapperLayout3));

export default NavbarWithSlices;
