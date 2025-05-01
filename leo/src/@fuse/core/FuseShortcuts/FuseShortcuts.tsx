import { amber } from '@mui/material/colors';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import Input from '@mui/material/Input';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import clsx from 'clsx';
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import Link from '@fuse/core/Link';
import _ from 'lodash';
import Box from '@mui/material/Box';
import FuseSvgIcon from '../FuseSvgIcon';
import { FuseFlatNavItemType } from '../FuseNavigation/types/FuseNavItemType';

type FuseShortcutsProps = {
	className?: string;
	navigation: FuseFlatNavItemType[];
	onChange: (T: string[]) => void;
	shortcuts?: string[];
	variant?: 'horizontal' | 'vertical';
};

/**
 * The FuseShortcuts component is responsible for rendering a list of shortcuts based on the navigation and shortcuts props.
 * It uses various MUI components to render the list items and search input.
 * The component is memoized to prevent unnecessary re-renders.
 */
function FuseShortcuts(props: FuseShortcutsProps) {
	const { navigation = [], shortcuts = [], onChange, variant = 'horizontal', className = '' } = props;

	const searchInputRef = useRef<HTMLInputElement>(null);
	const [addMenu, setAddMenu] = useState<HTMLElement | null>(null);
	const [searchText, setSearchText] = useState('');
	const [searchResults, setSearchResults] = useState<FuseFlatNavItemType[]>([]);
	const [shortcutItems, setShortcutItems] = useState<FuseFlatNavItemType[]>([]);

	useEffect(() => {
		const _shortcutItems = shortcuts
			? shortcuts.map((id) => _.find(navigation, { id }))
			: ([] as FuseFlatNavItemType[]);

		setShortcutItems(_shortcutItems);
	}, [navigation, shortcuts]);

	function addMenuClick(event: React.MouseEvent<HTMLElement>) {
		setAddMenu(event.currentTarget);
	}

	function addMenuClose() {
		setAddMenu(null);
	}

	function search(ev: React.ChangeEvent<HTMLInputElement>) {
		const newSearchText = ev.target.value;

		setSearchText(newSearchText);

		if (newSearchText.length !== 0 && navigation) {
			setSearchResults(
				navigation.filter((item) => item?.title?.toLowerCase()?.includes(newSearchText?.toLowerCase()))
			);
			return;
		}

		setSearchResults([]);
	}

	const toggleInShortcuts = useCallback(
		(id: string) => {
			let newShortcuts = [...shortcuts];

			newShortcuts = _.xor(newShortcuts, [id]);

			onChange(newShortcuts);
		},
		[onChange, shortcuts]
	);

	return (
		<Box className={clsx('flex shrink overflow-hidden', variant === 'vertical' ? 'flex-col' : '', className)}>
			{useMemo(() => {
				return (
					<Box
						className={clsx(
							'flex flex-1 items-center border rounded-lg',
							variant === 'vertical' ? 'flex-col' : 'max-h-9'
						)}
					>
						{shortcutItems.map(
							(_item) =>
								_item && (
									<Link
										to={_item.url}
										key={_item.id}
										role="button"
									>
										<Tooltip
											title={_item.title}
											placement={variant === 'horizontal' ? 'bottom' : 'left'}
										>
											<IconButton className="h-9 w-9 p-0 rounded-none">
												{_item.icon ? (
													<FuseSvgIcon size={20}>{_item.icon}</FuseSvgIcon>
												) : (
													<span className="text-2xl font-semibold uppercase">
														{_item.title[0]}
													</span>
												)}
											</IconButton>
										</Tooltip>
									</Link>
								)
						)}

						<Tooltip
							title="Click to add/remove shortcut"
							placement={variant === 'horizontal' ? 'bottom' : 'left'}
						>
							<IconButton
								className="h-9 w-9 p-0 rounded-none"
								aria-haspopup="true"
								onClick={addMenuClick}
							>
								<FuseSvgIcon
									size={20}
									sx={{ color: amber[600] }}
								>
									heroicons-solid:star
								</FuseSvgIcon>
							</IconButton>
						</Tooltip>
					</Box>
				);
			}, [variant, shortcutItems])}

			<Menu
				id="add-menu"
				anchorEl={addMenu}
				open={Boolean(addMenu)}
				onClose={addMenuClose}
				classes={{
					paper: 'min-w-64'
				}}
				TransitionProps={{
					onEntered: () => {
						searchInputRef?.current?.focus();
					},
					onExited: () => {
						setSearchText('');
					}
				}}
			>
				<div className="p-4 pt-2">
					<Input
						inputRef={searchInputRef}
						value={searchText}
						onChange={search}
						placeholder="Search for an app or page"
						fullWidth
						slotProps={{
							input: {
								'aria-label': 'Search'
							}
						}}
						disableUnderline
					/>
				</div>

				<Divider />

				{useMemo(() => {
					if (searchText.length === 0 || !searchResults || searchResults.length === 0) {
						return null;
					}

					return searchResults.map((_item) => (
						<ShortcutMenuItem
							shortcuts={shortcuts}
							key={_item.id}
							item={_item}
							onToggle={() => toggleInShortcuts(_item.id)}
						/>
					));
				}, [searchText.length, searchResults, shortcuts, toggleInShortcuts])}

				{searchText.length !== 0 && searchResults.length === 0 && (
					<Typography
						color="text.secondary"
						className="p-4 pb-2"
					>
						No results..
					</Typography>
				)}

				{useMemo(() => {
					if (searchText.length !== 0) {
						return null;
					}

					return shortcutItems.map(
						(_item) =>
							_item && (
								<ShortcutMenuItem
									shortcuts={shortcuts}
									key={_item.id}
									item={_item}
									onToggle={() => toggleInShortcuts(_item.id)}
								/>
							)
					);
				}, [searchText.length, shortcutItems, shortcuts, toggleInShortcuts])}
			</Menu>
		</Box>
	);
}

function ShortcutMenuItem(props: {
	shortcuts: FuseShortcutsProps['shortcuts'];
	item: FuseFlatNavItemType;
	onToggle: (T: string) => void;
}) {
	const { item, onToggle, shortcuts = [] } = props;

	if (!item || !item.id) {
		return null;
	}

	return (
		<Link
			to={item.url || ''}
			role="button"
		>
			<MenuItem key={item.id}>
				<ListItemIcon className="min-w-9">
					{item.icon ? (
						<FuseSvgIcon>{item.icon}</FuseSvgIcon>
					) : (
						<span className="text-center text-2xl font-semibold uppercase">{item.title[0]}</span>
					)}
				</ListItemIcon>
				<ListItemText primary={item.title} />
				<IconButton
					onClick={(ev) => {
						ev.preventDefault();
						ev.stopPropagation();
						onToggle(item.id);
					}}
					size="large"
				>
					<FuseSvgIcon color="action">
						{shortcuts.includes(item.id) ? 'heroicons-solid:star' : 'heroicons-outline:star'}
					</FuseSvgIcon>
				</IconButton>
			</MenuItem>
		</Link>
	);
}

export default memo(FuseShortcuts);
