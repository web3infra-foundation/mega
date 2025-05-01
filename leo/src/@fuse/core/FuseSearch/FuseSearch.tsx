import ClickAwayListener from '@mui/material/ClickAwayListener';
import { styled } from '@mui/material/styles';
import IconButton from '@mui/material/IconButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Popper from '@mui/material/Popper';
import TextField from '@mui/material/TextField';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import match from 'autosuggest-highlight/match';
import parse from 'autosuggest-highlight/parse';
import clsx from 'clsx';
import _ from 'lodash';
import { memo, useEffect, useReducer, useRef, ReactNode } from 'react';
import Autosuggest, { RenderInputComponentProps, RenderSuggestionParams, ChangeEvent } from 'react-autosuggest';
import * as React from 'react';
import useNavigate from '@fuse/hooks/useNavigate';
import FuseSvgIcon from '../FuseSvgIcon';
import { FuseFlatNavItemType } from '../FuseNavigation/types/FuseNavItemType';

const Root = styled('div')(({ theme }) => ({
	'& .FuseSearch-container': {
		position: 'relative'
	},
	'& .FuseSearch-suggestionsContainerOpen': {
		position: 'absolute',
		zIndex: 1,
		marginTop: theme.spacing(),
		left: 0,
		right: 0
	},
	'& .FuseSearch-suggestion': {
		display: 'block'
	},
	'& .FuseSearch-suggestionsList': {
		margin: 0,
		padding: 0,
		listStyleType: 'none'
	},
	'& .FuseSearch-input': {
		transition: theme.transitions.create(['background-color'], {
			easing: theme.transitions.easing.easeInOut,
			duration: theme.transitions.duration.short
		}),
		'&:focus': {
			backgroundColor: theme.vars.palette.background.paper
		}
	}
}));

type RenderInputComponentType = {
	variant?: 'basic' | 'standard';
	inputRef?: (node: HTMLInputElement) => void;
	ref?: (node: HTMLInputElement) => void;
	key?: string;
};

function renderInputComponent(props: RenderInputComponentProps) {
	const { variant, ref, inputRef = () => {}, key, ...other } = props as RenderInputComponentType;
	return (
		<div
			className="relative w-full"
			key={key}
		>
			{variant === 'basic' ? (
				// Outlined
				<>
					<TextField
						fullWidth
						autoComplete="off"
						slotProps={{
							input: {
								name: 'auto-complete-search',
								role: 'search',
								inputRef: (node: HTMLInputElement) => {
									ref?.(node);
									inputRef(node);
								},
								classes: {
									input: 'FuseSearch-input py-0 px-4 h-9 md:h-9 ltr:pr-9 rtl:pl-9',
									notchedOutline: 'rounded-lg'
								}
							}
						}}
						variant="outlined"
						{...other}
					/>
					<FuseSvgIcon
						className="pointer-events-none absolute top-0 h-9 w-9 p-2 ltr:right-0 rtl:left-0"
						color="action"
						size={20}
					>
						heroicons-outline:magnifying-glass
					</FuseSvgIcon>
				</>
			) : (
				// Standard
				<TextField
					fullWidth
					slotProps={{
						input: {
							disableUnderline: true,
							inputRef: (node: HTMLInputElement) => {
								ref?.(node);
								inputRef(node);
							},
							classes: {
								input: 'FuseSearch-input py-0 px-4 h-9'
							}
						}
					}}
					variant="standard"
					{...other}
				/>
			)}
		</div>
	);
}

function renderSuggestion(suggestion: FuseFlatNavItemType, { query, isHighlighted }: RenderSuggestionParams) {
	const matches = match(suggestion.title, query);
	const parts = parse(suggestion.title, matches);

	return (
		<MenuItem
			selected={Boolean(isHighlighted)}
			component="div"
		>
			<ListItemIcon className="min-w-9">
				{suggestion.icon ? (
					<FuseSvgIcon>{suggestion.icon}</FuseSvgIcon>
				) : (
					<span className="w-6 text-center text-2xl font-semibold uppercase">{suggestion.title[0]}</span>
				)}
			</ListItemIcon>
			<ListItemText
				primary={parts?.map((part: { text: string; highlight?: boolean }, index: number) =>
					part.highlight ? (
						<span
							key={index}
							style={{ fontWeight: 600 }}
						>
							{part.text}
						</span>
					) : (
						<strong
							key={index}
							style={{ fontWeight: 300 }}
						>
							{part.text}
						</strong>
					)
				)}
			/>
		</MenuItem>
	);
}

function getSuggestions(value: string, data: FuseFlatNavItemType[]): FuseFlatNavItemType[] {
	const inputValue = _.deburr(value.trim()).toLowerCase();
	const inputLength = inputValue.length;
	let count = 0;

	if (inputLength === 0) {
		return [];
	}

	return data.filter((suggestion) => {
		const keep = count < 10 && suggestion?.title && match(suggestion?.title, inputValue)?.length > 0;

		if (keep) {
			count += 1;
		}

		return keep;
	});
}

function getSuggestionValue(suggestion: FuseFlatNavItemType) {
	return suggestion.title;
}

type StateType = {
	searchText: string;
	search: boolean;
	navigation: FuseFlatNavItemType[];
	suggestions: FuseFlatNavItemType[];
	noSuggestions: boolean;
	opened: boolean;
};

const initialState: StateType = {
	searchText: '',
	search: false,
	navigation: [],
	suggestions: [],
	noSuggestions: false,
	opened: false
};

type ActionType =
	| { type: 'setSearchText'; value: string }
	| { type: 'setNavigation'; data: FuseFlatNavItemType[] }
	| { type: 'updateSuggestions'; value: string }
	| { type: 'clearSuggestions' }
	| { type: 'open' }
	| { type: 'close' };

function reducer(state: StateType, action: ActionType): StateType {
	switch (action.type) {
		case 'open': {
			return {
				...state,
				opened: true
			};
		}
		case 'close': {
			return {
				...state,
				opened: false,
				searchText: ''
			};
		}
		case 'setSearchText': {
			return {
				...state,
				searchText: action.value
			};
		}
		case 'setNavigation': {
			return {
				...state,
				navigation: action.data
			};
		}
		case 'updateSuggestions': {
			const suggestions = getSuggestions(action.value, state.navigation);
			const isInputBlank = typeof action.value === 'string' && action.value.trim() === '';
			const noSuggestions = !isInputBlank && suggestions.length === 0;

			return {
				...state,
				suggestions,
				noSuggestions
			};
		}
		case 'clearSuggestions': {
			return {
				...state,
				suggestions: [],
				noSuggestions: false
			};
		}
		default: {
			throw new Error();
		}
	}
}

/**
 * Props for FuseSearch component
 */
type FuseSearchProps = {
	className?: string;
	navigation: FuseFlatNavItemType[];
	variant?: 'basic' | 'full';
	trigger?: ReactNode;
	placeholder?: string;
	noResults?: string;
};

/**
 * FuseSearch component
 */
function FuseSearch(props: FuseSearchProps) {
	const {
		navigation = [],
		className,
		variant = 'full',
		placeholder = 'Search',
		noResults = 'No results..',
		trigger = (
			<IconButton className="border border-divider">
				<FuseSvgIcon size={20}>heroicons-outline:magnifying-glass</FuseSvgIcon>
			</IconButton>
		)
	} = props;
	const navigate = useNavigate();

	const [state, dispatch] = useReducer(reducer, initialState);

	const suggestionsNode = useRef<HTMLDivElement>(null);
	const popperNode = useRef<HTMLDivElement>(null);
	const buttonNode = useRef(null);

	useEffect(() => {
		dispatch({
			type: 'setNavigation',
			data: navigation
		});
	}, [navigation]);

	function showSearch() {
		dispatch({ type: 'open' });
		document.addEventListener('keydown', escFunction, false);
	}

	function hideSearch() {
		dispatch({ type: 'close' });
		document.removeEventListener('keydown', escFunction, false);
	}

	function escFunction(event: KeyboardEvent) {
		if (event.key === 'Esc' || event.key === 'Escape') {
			hideSearch();
		}
	}

	function handleSuggestionsFetchRequested({ value }: { value: string }) {
		dispatch({
			type: 'updateSuggestions',
			value
		});
	}

	function handleSuggestionSelected(
		event: React.FormEvent<unknown>,
		{ suggestion }: { suggestion: FuseFlatNavItemType }
	) {
		event.preventDefault();
		event.stopPropagation();

		if (!suggestion.url) {
			return;
		}

		hideSearch();

		navigate(suggestion.url);
	}

	function handleSuggestionsClearRequested() {
		dispatch({
			type: 'clearSuggestions'
		});
	}

	function handleChange(_event: React.FormEvent<HTMLElement>, { newValue }: ChangeEvent) {
		dispatch({
			type: 'setSearchText',
			value: newValue
		});
	}

	function handleClickAway(event: MouseEvent | TouchEvent) {
		if (
			state.opened &&
			(!suggestionsNode.current ||
				!(event.target instanceof Node && suggestionsNode.current.contains(event.target)))
		) {
			hideSearch();
		}
	}

	switch (variant) {
		case 'basic': {
			return (
				<div
					className={clsx('flex w-full items-center', className)}
					ref={popperNode}
				>
					<Autosuggest
						renderInputComponent={renderInputComponent}
						highlightFirstSuggestion
						suggestions={state.suggestions}
						onSuggestionsFetchRequested={handleSuggestionsFetchRequested}
						onSuggestionsClearRequested={handleSuggestionsClearRequested}
						onSuggestionSelected={handleSuggestionSelected}
						getSuggestionValue={getSuggestionValue}
						renderSuggestion={renderSuggestion}
						inputProps={{
							// eslint-disable-next-line @typescript-eslint/ban-ts-comment
							// @ts-ignore
							variant,
							placeholder,
							role: 'search',
							value: state.searchText,
							onChange: handleChange,
							onFocus: showSearch,
							InputLabelProps: {
								shrink: true
							},
							autoFocus: false
						}}
						theme={{
							container: 'flex flex-1 w-full',
							suggestionsList: 'FuseSearch-suggestionsList',
							suggestion: 'FuseSearch-suggestion'
						}}
						renderSuggestionsContainer={(options) => {
							const { containerProps } = options;
							const { key, ...restContainerProps } = containerProps;

							return (
								<Popper
									anchorEl={popperNode.current}
									open={Boolean(options.children) || state.noSuggestions}
									className="z-9999"
								>
									<div ref={suggestionsNode}>
										<Paper
											key={key}
											{...restContainerProps}
											style={{
												width: popperNode.current ? popperNode.current.clientWidth : ''
											}}
											className="overflow-hidden rounded-lg shadow-lg"
										>
											{options.children}
											{state.noSuggestions && (
												<Typography className="px-4 py-3">{noResults}</Typography>
											)}
										</Paper>
									</div>
								</Popper>
							);
						}}
					/>
				</div>
			);
		}
		case 'full': {
			return (
				<Root className={clsx('flex', className)}>
					<Tooltip
						title="Click to search"
						placement="bottom"
					>
						<div
							onClick={showSearch}
							onKeyDown={showSearch}
							role="button"
							tabIndex={0}
							ref={buttonNode}
						>
							{trigger}
						</div>
					</Tooltip>

					{state.opened && (
						<ClickAwayListener onClickAway={handleClickAway}>
							<Paper
								className="absolute inset-x-0 top-0 z-9999 h-full shadow-0"
								square
							>
								<div
									className="flex h-full w-full items-center"
									ref={popperNode}
								>
									<Autosuggest
										renderInputComponent={renderInputComponent}
										highlightFirstSuggestion
										suggestions={state.suggestions}
										onSuggestionsFetchRequested={handleSuggestionsFetchRequested}
										onSuggestionsClearRequested={handleSuggestionsClearRequested}
										onSuggestionSelected={handleSuggestionSelected}
										getSuggestionValue={getSuggestionValue}
										renderSuggestion={renderSuggestion}
										inputProps={{
											placeholder,
											value: state.searchText,
											onChange: handleChange,
											// eslint-disable-next-line @typescript-eslint/ban-ts-comment
											// @ts-ignore
											InputLabelProps: {
												shrink: true
											},
											autoFocus: true
										}}
										theme={{
											container: 'flex flex-1 w-full',
											suggestionsList: 'FuseSearch-suggestionsList',
											suggestion: 'FuseSearch-suggestion'
										}}
										renderSuggestionsContainer={(options) => {
											const { containerProps } = options;
											const { key, ...restContainerProps } = containerProps;

											return (
												<Popper
													anchorEl={popperNode.current}
													open={Boolean(options.children) || state.noSuggestions}
													className="z-9999"
												>
													<div ref={suggestionsNode}>
														<Paper
															square
															key={key}
															{...restContainerProps}
															className="shadow-lg"
															style={{
																width: popperNode.current
																	? popperNode.current.clientWidth
																	: 'auto'
															}}
														>
															{options.children}
															{state.noSuggestions && (
																<Typography className="px-4 py-3">
																	{noResults}
																</Typography>
															)}
														</Paper>
													</div>
												</Popper>
											);
										}}
									/>
									<IconButton
										onClick={hideSearch}
										className="mx-2"
										size="large"
									>
										<FuseSvgIcon>heroicons-outline:x-mark</FuseSvgIcon>
									</IconButton>
								</div>
							</Paper>
						</ClickAwayListener>
					)}
				</Root>
			);
		}
		default: {
			return null;
		}
	}
}

export default memo(FuseSearch);
