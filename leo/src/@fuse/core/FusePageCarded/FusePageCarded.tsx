'use client';

import FuseScrollbars from '@fuse/core/FuseScrollbars';
import { styled } from '@mui/material/styles';
import clsx from 'clsx';
import { memo, ReactNode, useImperativeHandle, useRef, RefObject } from 'react';
import GlobalStyles from '@mui/material/GlobalStyles';
import { SystemStyleObject, Theme } from '@mui/system';
import FusePageCardedSidebar from './FusePageCardedSidebar';
import FusePageCardedHeader from './FusePageCardedHeader';
import { FuseScrollbarsProps } from '../FuseScrollbars/FuseScrollbars';

const headerHeight = 120;
const toolbarHeight = 64;

type FusePageCardedProps = SystemStyleObject<Theme> & {
	className?: string;
	leftSidebarContent?: ReactNode;
	leftSidebarVariant?: 'permanent' | 'persistent' | 'temporary';
	rightSidebarContent?: ReactNode;
	rightSidebarVariant?: 'permanent' | 'persistent' | 'temporary';
	header?: ReactNode;
	content?: ReactNode;
	scroll?: 'normal' | 'page' | 'content';
	leftSidebarOpen?: boolean;
	rightSidebarOpen?: boolean;
	leftSidebarWidth?: number;
	rightSidebarWidth?: number;
	rightSidebarOnClose?: () => void;
	leftSidebarOnClose?: () => void;
	contentScrollbarsProps?: FuseScrollbarsProps;
	ref?: RefObject<{ toggleLeftSidebar: (val: boolean) => void; toggleRightSidebar: (val: boolean) => void }>;
};

const Root = styled('div')<FusePageCardedProps>(({ theme, ...props }) => ({
	display: 'flex',
	flexDirection: 'column',
	minWidth: 0,
	minHeight: '100%',
	position: 'relative',
	flex: '1 1 auto',
	width: '100%',
	height: 'auto',
	padding: '0 16px',
	backgroundColor: theme.vars.palette.background.default,

	'& .FusePageCarded-scroll-content': {
		height: '100%'
	},

	'& .FusePageCarded-wrapper': {
		display: 'flex',
		flexDirection: 'row',
		flex: '1 1 auto',
		zIndex: 2,
		maxWidth: '100%',
		minWidth: 0,
		height: '100%',
		backgroundColor: theme.vars.palette.background.paper,

		...(props.scroll === 'content' && {
			position: 'absolute',
			top: 0,
			bottom: 0,
			right: 0,
			left: 0,
			overflow: 'hidden'
		})
	},

	'& .FusePageCarded-header': {
		display: 'flex',
		flex: '0 0 auto'
	},

	'& .FusePageCarded-contentWrapper': {
		display: 'flex',
		flexDirection: 'column',
		flex: '1 1 auto',
		overflow: 'auto',
		WebkitOverflowScrolling: 'touch',
		zIndex: 9999
	},

	'& .FusePageCarded-toolbar': {
		height: toolbarHeight,
		minHeight: toolbarHeight,
		display: 'flex',
		alignItems: 'center'
	},

	'& .FusePageCarded-content': {
		flex: '1 0 auto'
	},

	'& .FusePageCarded-sidebarWrapper': {
		overflow: 'hidden',
		backgroundColor: 'transparent',
		position: 'absolute',
		'&.permanent': {
			[theme.breakpoints.up('lg')]: {
				position: 'relative',
				marginLeft: 0,
				marginRight: 0,
				transition: theme.transitions.create('margin', {
					easing: theme.transitions.easing.sharp,
					duration: theme.transitions.duration.leavingScreen
				}),
				'&.closed': {
					transition: theme.transitions.create('margin', {
						easing: theme.transitions.easing.easeOut,
						duration: theme.transitions.duration.enteringScreen
					}),

					'&.FusePageCarded-leftSidebar': {
						marginLeft: -props.leftSidebarWidth
					},
					'&.FusePageCarded-rightSidebar': {
						marginRight: -props.rightSidebarWidth
					}
				}
			}
		}
	},

	'& .FusePageCarded-sidebar': {
		position: 'absolute',
		backgroundColor: theme.vars.palette.background.paper,
		color: theme.vars.palette.text.primary,

		'&.permanent': {
			[theme.breakpoints.up('lg')]: {
				position: 'relative'
			}
		},
		maxWidth: '100%',
		height: '100%'
	},

	'& .FusePageCarded-leftSidebar': {
		width: props.leftSidebarWidth,

		[theme.breakpoints.up('lg')]: {
			// borderRight: `1px solid ${theme.vars.palette.divider}`,
			// borderLeft: 0,
		}
	},

	'& .FusePageCarded-rightSidebar': {
		width: props.rightSidebarWidth,

		[theme.breakpoints.up('lg')]: {
			// borderLeft: `1px solid ${theme.vars.palette.divider}`,
			// borderRight: 0,
		}
	},

	'& .FusePageCarded-sidebarHeader': {
		height: headerHeight,
		minHeight: headerHeight,
		backgroundColor: theme.vars.palette.primary.dark,
		color: theme.vars.palette.primary.contrastText
	},

	'& .FusePageCarded-sidebarHeaderInnerSidebar': {
		backgroundColor: 'transparent',
		color: 'inherit',
		height: 'auto',
		minHeight: 'auto'
	},

	'& .FusePageCarded-sidebarContent': {
		display: 'flex',
		flexDirection: 'column',
		minHeight: '100%'
	},

	'& .FusePageCarded-backdrop': {
		position: 'absolute'
	}
}));

function FusePageCarded(props: FusePageCardedProps) {
	const {
		scroll = 'page',
		className,
		header,
		content,
		leftSidebarContent,
		rightSidebarContent,
		leftSidebarOpen = false,
		rightSidebarOpen = false,
		rightSidebarWidth = 240,
		leftSidebarWidth = 240,
		leftSidebarVariant = 'permanent',
		rightSidebarVariant = 'permanent',
		rightSidebarOnClose,
		leftSidebarOnClose,
		contentScrollbarsProps,
		ref
	} = props;

	const leftSidebarRef = useRef<{ toggleSidebar: (T: boolean) => void }>(null);
	const rightSidebarRef = useRef<{ toggleSidebar: (T: boolean) => void }>(null);
	const rootRef = useRef(null);

	useImperativeHandle(ref, () => ({
		toggleLeftSidebar: (val: boolean) => {
			if (leftSidebarRef.current) {
				leftSidebarRef.current.toggleSidebar(val);
			}
		},
		toggleRightSidebar: (val: boolean) => {
			if (rightSidebarRef.current) {
				rightSidebarRef.current.toggleSidebar(val);
			}
		}
	}));

	return (
		<>
			<GlobalStyles
				styles={() => ({
					...(scroll !== 'page' && {
						'#fuse-toolbar': {
							position: 'static!important'
						},
						'#fuse-footer': {
							position: 'static!important'
						}
					}),
					...(scroll === 'page' && {
						'#fuse-toolbar': {
							position: 'sticky',
							top: 0
						},
						'#fuse-footer': {
							position: 'sticky',
							bottom: 0
						}
					})
				})}
			/>
			<Root
				className={clsx('FusePageCarded-root', `FusePageCarded-scroll-${scroll}`, className)}
				ref={rootRef}
				scroll={scroll}
				leftSidebarWidth={leftSidebarWidth}
				rightSidebarWidth={rightSidebarWidth}
			>
				{header && <FusePageCardedHeader header={header} />}

				<div className="container relative z-10 flex h-full flex-auto flex-col overflow-hidden rounded-t-lg shadow-1">
					<div className="FusePageCarded-wrapper">
						{leftSidebarContent && (
							<FusePageCardedSidebar
								position="left"
								variant={leftSidebarVariant}
								ref={leftSidebarRef}
								open={leftSidebarOpen}
								onClose={leftSidebarOnClose}
								width={leftSidebarWidth}
							>
								{leftSidebarContent}
							</FusePageCardedSidebar>
						)}
						<FuseScrollbars
							className="FusePageCarded-contentWrapper"
							enable={scroll === 'content'}
							{...contentScrollbarsProps}
						>
							{content && <div className={clsx('FusePageCarded-content')}>{content}</div>}
						</FuseScrollbars>
						{rightSidebarContent && (
							<FusePageCardedSidebar
								position="right"
								variant={rightSidebarVariant || 'permanent'}
								ref={rightSidebarRef}
								open={rightSidebarOpen}
								onClose={rightSidebarOnClose}
								width={rightSidebarWidth}
							>
								{rightSidebarContent}
							</FusePageCardedSidebar>
						)}
					</div>
				</div>
			</Root>
		</>
	);
}

const StyledFusePageCarded = memo(styled(FusePageCarded)``);

export default StyledFusePageCarded;
