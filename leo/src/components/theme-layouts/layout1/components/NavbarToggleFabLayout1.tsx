import { useAppDispatch } from 'src/store/hooks';
import useThemeMediaQuery from '@fuse/hooks/useThemeMediaQuery';
import { navbarToggle, navbarToggleMobile } from 'src/components/theme-layouts/components/navbar/navbarSlice';
import NavbarToggleFab from 'src/components/theme-layouts/components/navbar/NavbarToggleFab';
import useFuseLayoutSettings from '@fuse/core/FuseLayout/useFuseLayoutSettings';
import { Layout1ConfigDefaultsType } from '@/components/theme-layouts/layout1/Layout1Config';

type NavbarToggleFabLayout1Props = {
	className?: string;
};

/**
 * The navbar toggle fab layout 1.
 */
function NavbarToggleFabLayout1(props: NavbarToggleFabLayout1Props) {
	const { className } = props;

	const isMobile = useThemeMediaQuery((theme) => theme.breakpoints.down('lg'));

	const settings = useFuseLayoutSettings();
	const config = settings.config as Layout1ConfigDefaultsType;

	const dispatch = useAppDispatch();

	return (
		<NavbarToggleFab
			className={className}
			onClick={() => {
				dispatch(isMobile ? navbarToggleMobile() : navbarToggle());
			}}
			position={config.navbar.position}
		/>
	);
}

export default NavbarToggleFabLayout1;
