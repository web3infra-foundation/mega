import AppBar from '@mui/material/AppBar';
import { ThemeProvider } from '@mui/material/styles';
import Toolbar from '@mui/material/Toolbar';
import { memo } from 'react';
import clsx from 'clsx';
import { useFooterTheme } from '@fuse/core/FuseSettings/hooks/fuseThemeHooks';

type FooterLayout1Props = { className?: string };

/**
 * The footer layout 1.
 */
function FooterLayout1(props: FooterLayout1Props) {
	const { className } = props;

	const footerTheme = useFooterTheme();

	return (
		<ThemeProvider theme={footerTheme}>
			<AppBar
				id="fuse-footer"
				className={clsx('relative z-20 border-t', className)}
				color="default"
				sx={(theme) => ({
					backgroundColor: footerTheme.palette.background.default,
					...theme.applyStyles('light', {
						backgroundColor: footerTheme.palette.background.paper
					})
				})}
				elevation={0}
			>
				<Toolbar className="min-h-12 md:min-h-16 px-2 sm:px-3 py-0 flex items-center overflow-x-auto">
					Footer
				</Toolbar>
			</AppBar>
		</ThemeProvider>
	);
}

export default memo(FooterLayout1);
