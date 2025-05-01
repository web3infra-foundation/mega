import Avatar from '@mui/material/Avatar';
import Button from '@mui/material/Button';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import MenuItem from '@mui/material/MenuItem';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import Link from '@fuse/core/Link';
import FuseSvgIcon from '@fuse/core/FuseSvgIcon';
import { darken } from '@mui/material/styles';
import Tooltip from '@mui/material/Tooltip';
import clsx from 'clsx';
import Popover, { PopoverProps } from '@mui/material/Popover';
import useUser from '@auth/useUser';

type UserMenuProps = {
	className?: string;
	popoverProps?: Partial<PopoverProps>;
	arrowIcon?: string;
};

/**
 * The user menu.
 */
function UserMenu(props: UserMenuProps) {
	const { className, popoverProps, arrowIcon = 'heroicons-outline:chevron-up' } = props;
	const { data: user, signOut, isGuest } = useUser();
	const [userMenu, setUserMenu] = useState<HTMLElement | null>(null);
	const userMenuClick = (event: React.MouseEvent<HTMLElement>) => {
		setUserMenu(event.currentTarget);
	};

	const userMenuClose = () => {
		setUserMenu(null);
	};

	if (!user) {
		return null;
	}

	return (
		<>
			<Button
				className={clsx(
					'user-menu flex justify-start shrink-0 min-h-14 h-14 rounded-lg p-2 space-x-3',
					className
				)}
				sx={(theme) => ({
					borderColor: theme.vars.palette.divider,
					'&:hover, &:focus': {
						backgroundColor: `rgba(${theme.vars.palette.dividerChannel} / 0.6)`,
						...theme.applyStyles('dark', {
							backgroundColor: `rgba(${theme.vars.palette.dividerChannel} / 0.1)`
						})
					}
				})}
				onClick={userMenuClick}
				color="inherit"
			>
				{user?.photoURL ? (
					<Avatar
						sx={(theme) => ({
							background: theme.vars.palette.background.default,
							color: theme.vars.palette.text.secondary
						})}
						className="avatar w-10 h-10 rounded-lg"
						alt="user photo"
						src={user?.photoURL}
						variant="rounded"
					/>
				) : (
					<Avatar
						sx={(theme) => ({
							background: (theme) => darken(theme.palette.background.default, 0.05),
							color: theme.vars.palette.text.secondary
						})}
						className="avatar md:mx-1"
					>
						{user?.displayName?.[0]}
					</Avatar>
				)}
				<div className="flex flex-col flex-auto space-y-2">
					<Typography
						component="span"
						className="title flex font-semibold text-base capitalize truncate tracking-tight leading-none"
					>
						{user?.displayName}
					</Typography>
					<Typography
						className="subtitle flex text-md font-medium tracking-tighter leading-none"
						color="text.secondary"
					>
						{user?.email}
					</Typography>
				</div>
				<div className="flex shrink-0 items-center space-x-2">
					<Tooltip
						title={
							<>
								{user.role?.toString()}
								{(!user.role || (Array.isArray(user.role) && user.role.length === 0)) && 'Guest'}
							</>
						}
					>
						<FuseSvgIcon
							className="info-icon"
							size={20}
						>
							heroicons-outline:information-circle
						</FuseSvgIcon>
					</Tooltip>
					<FuseSvgIcon
						className="arrow"
						size={13}
					>
						{arrowIcon}
					</FuseSvgIcon>
				</div>
			</Button>
			<Popover
				open={Boolean(userMenu)}
				anchorEl={userMenu}
				onClose={userMenuClose}
				anchorOrigin={{
					vertical: 'top',
					horizontal: 'center'
				}}
				transformOrigin={{
					vertical: 'bottom',
					horizontal: 'center'
				}}
				classes={{
					paper: 'py-2 min-w-64'
				}}
				{...popoverProps}
			>
				{isGuest ? (
					<>
						<MenuItem
							component={Link}
							to="/sign-in"
							role="button"
						>
							<ListItemIcon className="min-w-9">
								<FuseSvgIcon>heroicons-outline:lock-closed</FuseSvgIcon>
							</ListItemIcon>
							<ListItemText primary="Sign In" />
						</MenuItem>
						<MenuItem
							component={Link}
							to="/sign-up"
							role="button"
						>
							<ListItemIcon className="min-w-9">
								<FuseSvgIcon>heroicons-outline:user-plus</FuseSvgIcon>
							</ListItemIcon>
							<ListItemText primary="Sign up" />
						</MenuItem>
					</>
				) : (
					<>
						<MenuItem
							component={Link}
							to="/apps/profile"
							onClick={userMenuClose}
							role="button"
						>
							<ListItemIcon className="min-w-9">
								<FuseSvgIcon>heroicons-outline:user-circle</FuseSvgIcon>
							</ListItemIcon>
							<ListItemText primary="My Profile" />
						</MenuItem>
						<MenuItem
							component={Link}
							to="/apps/mailbox"
							onClick={userMenuClose}
							role="button"
						>
							<ListItemIcon className="min-w-9">
								<FuseSvgIcon>heroicons-outline:envelope</FuseSvgIcon>
							</ListItemIcon>
							<ListItemText primary="Inbox" />
						</MenuItem>
						<MenuItem
							onClick={() => {
								signOut();
							}}
						>
							<ListItemIcon className="min-w-9">
								<FuseSvgIcon>heroicons-outline:arrow-right-on-rectangle</FuseSvgIcon>
							</ListItemIcon>
							<ListItemText primary="Sign out" />
						</MenuItem>
					</>
				)}
			</Popover>
		</>
	);
}

export default UserMenu;
