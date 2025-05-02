import FuseScrollbars from '@fuse/core/FuseScrollbars';
import { styled } from '@mui/material/styles';
import Fab from '@mui/material/Fab';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import SwipeableDrawer from '@mui/material/SwipeableDrawer';
import Tooltip from '@mui/material/Tooltip';
import clsx from 'clsx';
import { memo, ReactNode, useState } from 'react';
import FuseSvgIcon from '../FuseSvgIcon';
import useThemeMediaQuery from '../../hooks/useThemeMediaQuery';

const Root = styled('div')(({ theme }) => ({
	'& .FuseSidePanel-paper': {
		display: 'flex',
		width: 56,
		transition: theme.transitions.create(['transform', 'width', 'min-width'], {
			easing: theme.transitions.easing.sharp,
			duration: theme.transitions.duration.shorter
		}),
		paddingBottom: 64,
		height: '100%',
		maxHeight: '100vh',
		position: 'sticky',
		top: 0,
		zIndex: 999,
		'&.left': {
			'& .FuseSidePanel-buttonWrapper': {
				left: 0,
				right: 'auto'
			},
			'& .FuseSidePanel-buttonIcon': {
				transform: 'rotate(0deg)'
			}
		},
		'&.right': {
			'& .FuseSidePanel-buttonWrapper': {
				right: 0,
				left: 'auto'
			},
			'& .FuseSidePanel-buttonIcon': {
				transform: 'rotate(-180deg)'
			}
		},
		'&.closed': {
			[theme.breakpoints.up('lg')]: {
				width: 0
			},
			'&.left': {
				'& .FuseSidePanel-buttonWrapper': {
					justifyContent: 'start'
				},
				'& .FuseSidePanel-button': {
					borderBottomLeftRadius: 0,
					borderTopLeftRadius: 0,
					paddingLeft: 4
				},
				'& .FuseSidePanel-buttonIcon': {
					transform: 'rotate(-180deg)'
				}
			},
			'&.right': {
				'& .FuseSidePanel-buttonWrapper': {
					justifyContent: 'flex-end'
				},
				'& .FuseSidePanel-button': {
					borderBottomRightRadius: 0,
					borderTopRightRadius: 0,
					paddingRight: 4
				},
				'& .FuseSidePanel-buttonIcon': {
					transform: 'rotate(0deg)'
				}
			},
			'& .FuseSidePanel-buttonWrapper': {
				width: 'auto'
			},
			'& .FuseSidePanel-button': {
				backgroundColor: theme.vars.palette.background.paper,
				borderRadius: 38,
				transition: theme.transitions.create(
					['background-color', 'border-radius', 'width', 'min-width', 'padding'],
					{
						easing: theme.transitions.easing.easeInOut,
						duration: theme.transitions.duration.shorter
					}
				),
				width: 24,
				'&:hover': {
					width: 52,
					paddingLeft: 8,
					paddingRight: 8
				}
			},
			'& .FuseSidePanel-content': {
				opacity: 0
			}
		}
	},
	'& .FuseSidePanel-content': {
		overflow: 'hidden',
		opacity: 1,
		transition: theme.transitions.create(['opacity'], {
			easing: theme.transitions.easing.easeInOut,
			duration: theme.transitions.duration.short
		})
	},
	'& .FuseSidePanel-buttonWrapper': {
		position: 'absolute',
		bottom: 0,
		left: 0,
		display: 'flex',
		alignItems: 'center',
		justifyContent: 'center',
		padding: '12px 0',
		width: '100%',
		minWidth: 56
	},
	'& .FuseSidePanel-button': {
		padding: 8,
		width: 40,
		height: 40
	},
	'& .FuseSidePanel-buttonIcon': {
		transition: theme.transitions.create(['transform'], {
			easing: theme.transitions.easing.easeInOut,
			duration: theme.transitions.duration.short
		})
	},
	'& .FuseSidePanel-mobileButton': {
		height: 40,
		position: 'fixed',
		zIndex: 99,
		bottom: 12,
		width: 24,
		borderRadius: 38,
		padding: 8,
		backgroundColor: theme.vars.palette.background.paper,
		transition: theme.transitions.create(['background-color', 'border-radius', 'width', 'min-width', 'padding'], {
			easing: theme.transitions.easing.easeInOut,
			duration: theme.transitions.duration.shorter
		}),
		'&:hover': {
			width: 52,
			paddingLeft: 8,
			paddingRight: 8
		},
		'&.left': {
			borderBottomLeftRadius: 0,
			borderTopLeftRadius: 0,
			paddingLeft: 4,
			left: 0
		},
		'&.right': {
			borderBottomRightRadius: 0,
			borderTopRightRadius: 0,
			paddingRight: 4,
			right: 0,
			'& .FuseSidePanel-buttonIcon': {
				transform: 'rotate(-180deg)'
			}
		}
	}
}));

type FuseSidePanelProps = {
	position?: 'left';
	opened?: true;
	className?: string;
	children?: ReactNode;
};

/**
 * The FuseSidePanel component is responsible for rendering a side panel that can be opened and closed.
 * It uses various MUI components to render the panel and its contents.
 * The component is memoized to prevent unnecessary re-renders.
 */
function FuseSidePanel(props: FuseSidePanelProps) {
	const { position = 'left', opened = true, className, children } = props;
	const isMobile = useThemeMediaQuery((theme) => theme.breakpoints.down('lg'));

	const [panelOpened, setPanelOpened] = useState(Boolean(opened));
	const [mobileOpen, setMobileOpen] = useState(false);

	function toggleOpened() {
		setPanelOpened(!panelOpened);
	}

	function toggleMobileDrawer() {
		setMobileOpen(!mobileOpen);
	}

	return (
		<Root>
			{!isMobile && (
				<Paper
					className={clsx(
						'FuseSidePanel-paper',
						className,
						panelOpened ? 'opened' : 'closed',
						position,
						'shadow-lg'
					)}
					square
				>
					<FuseScrollbars className={clsx('content', 'FuseSidePanel-content')}>{children}</FuseScrollbars>

					<div className="FuseSidePanel-buttonWrapper">
						<Tooltip
							title="Toggle side panel"
							placement={position === 'left' ? 'right' : 'right'}
						>
							<IconButton
								className="FuseSidePanel-button"
								onClick={toggleOpened}
								disableRipple
								size="large"
							>
								<FuseSvgIcon className="FuseSidePanel-buttonIcon">
									heroicons-outline:chevron-left
								</FuseSvgIcon>
							</IconButton>
						</Tooltip>
					</div>
				</Paper>
			)}

			{isMobile && (
				<>
					<SwipeableDrawer
						classes={{
							paper: clsx('FuseSidePanel-paper', className)
						}}
						anchor={position}
						open={mobileOpen}
						onOpen={() => {}}
						onClose={toggleMobileDrawer}
						disableSwipeToOpen
					>
						<FuseScrollbars className={clsx('content', 'FuseSidePanel-content')}>{children}</FuseScrollbars>
					</SwipeableDrawer>

					<Tooltip
						title="Hide side panel"
						placement={position === 'left' ? 'right' : 'right'}
					>
						<Fab
							className={clsx('FuseSidePanel-mobileButton', position)}
							onClick={toggleMobileDrawer}
							disableRipple
						>
							<FuseSvgIcon className="FuseSidePanel-buttonIcon">
								heroicons-outline:chevron-right
							</FuseSvgIcon>
						</Fab>
					</Tooltip>
				</>
			)}
		</Root>
	);
}

export default memo(FuseSidePanel);
