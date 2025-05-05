import FuseScrollbars from '@fuse/core/FuseScrollbars';
import { styled } from '@mui/material/styles';
import clsx from 'clsx';
import { memo } from 'react';
import { Divider } from '@mui/material';
import UserMenu from 'src/components/theme-layouts/components/UserMenu';
import Logo from '../../components/Logo';
import Navigation from '../../components/navigation/Navigation';
import GoToDocBox from '../../components/GoToDocBox';

const Root = styled('div')(({ theme }) => ({
	backgroundColor: theme.vars.palette.background.default,
	color: theme.vars.palette.text.primary,
	'& ::-webkit-scrollbar-thumb': {
		boxShadow: `inset 0 0 0 20px ${'rgba(255, 255, 255, 0.24)'}`,
		...theme.applyStyles('light', {
			boxShadow: `inset 0 0 0 20px ${'rgba(0, 0, 0, 0.24)'}`
		})
	},
	'& ::-webkit-scrollbar-thumb:active': {
		boxShadow: `inset 0 0 0 20px ${'rgba(255, 255, 255, 0.37)'}`,
		...theme.applyStyles('light', {
			boxShadow: `inset 0 0 0 20px ${'rgba(0, 0, 0, 0.37)'}`
		})
	}
}));

const StyledContent = styled(FuseScrollbars)(() => ({
	overscrollBehavior: 'contain',
	overflowX: 'hidden',
	overflowY: 'auto',
	WebkitOverflowScrolling: 'touch',
	backgroundRepeat: 'no-repeat',
	backgroundSize: '100% 40px, 100% 10px',
	backgroundAttachment: 'local, scroll'
}));

type NavbarMobileLayout2Props = {
	className?: string;
};

/**
 * The navbar mobile layout 2.
 */
function NavbarMobileLayout2(props: NavbarMobileLayout2Props) {
	const { className = '' } = props;

	return (
		<Root className={clsx('flex h-full flex-col overflow-hidden', className)}>
			<div className="flex h-12 shrink-0 flex-row items-center px-3 md:h-18">
				<Logo />
			</div>

			<StyledContent
				className="flex min-h-0 flex-1 flex-col"
				option={{ suppressScrollX: true, wheelPropagation: false }}
			>
				<Navigation layout="vertical" />

				<div className="shrink-0 flex items-center justify-center py-12 opacity-10">
					<img
						className="w-full max-w-16"
						src="/assets/images/logo/logo.svg"
						alt="footer logo"
					/>
				</div>
			</StyledContent>

			<GoToDocBox className="mx-3 my-4" />

			<Divider />

			<div className="p-1 md:p-4 w-full">
				<UserMenu className="w-full" />
			</div>
		</Root>
	);
}

export default memo(NavbarMobileLayout2);
