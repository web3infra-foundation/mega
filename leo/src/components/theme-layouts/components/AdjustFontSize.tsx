'use client';

import { MouseEvent, useState } from 'react';
import Slider from '@mui/material/Slider';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Menu from '@mui/material/Menu';
import FuseSvgIcon from '@fuse/core/FuseSvgIcon';
import clsx from 'clsx';

const marks = [
	{ value: 16 * 0.7, label: '70%' },
	{ value: 16 * 0.8, label: '80%' },
	{ value: 16 * 0.9, label: '90%' },
	{ value: 16, label: '100%' },
	{ value: 16 * 1.1, label: '110%' },
	{ value: 16 * 1.2, label: '120%' },
	{ value: 16 * 1.3, label: '130%' }
];

type AdjustFontSizeProps = {
	className?: string;
};

/**
 * The adjust font size.
 */
function AdjustFontSize(props: AdjustFontSizeProps) {
	const { className = '' } = props;

	const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
	const [fontSize, setFontSize] = useState(16);

	function changeHtmlFontSize() {
		const html = document.getElementsByTagName('html')[0];
		html.style.fontSize = `${fontSize}px`;
	}

	const handleClick = (event: MouseEvent<HTMLElement>) => {
		setAnchorEl(event.currentTarget);
	};

	const handleClose = () => {
		setAnchorEl(null);
	};

	return (
		<div>
			<IconButton
				className={clsx('border border-divider', className)}
				aria-controls="font-size-menu"
				aria-haspopup="true"
				onClick={handleClick}
			>
				<FuseSvgIcon size={20}>material-outline:format_size</FuseSvgIcon>
			</IconButton>
			<Menu
				classes={{ paper: 'w-80' }}
				id="font-size-menu"
				anchorEl={anchorEl}
				keepMounted
				open={Boolean(anchorEl)}
				onClose={handleClose}
				anchorOrigin={{
					vertical: 'bottom',
					horizontal: 'center'
				}}
				transformOrigin={{
					vertical: 'top',
					horizontal: 'center'
				}}
			>
				<div className="px-6 py-3">
					<Typography className="mb-2 flex items-center justify-center text-lg font-semibold">
						<FuseSvgIcon
							color="action"
							className="mr-1"
						>
							material-outline:format_size
						</FuseSvgIcon>
						Font Size
					</Typography>
					<Slider
						classes={{ markLabel: 'text-md font-semibold' }}
						value={fontSize}
						track={false}
						aria-labelledby="discrete-slider-small-steps"
						marks={marks}
						min={16 * 0.7}
						max={16 * 1.3}
						step={null}
						valueLabelDisplay="off"
						onChange={(ev, value) => setFontSize(value as number)}
						onChangeCommitted={changeHtmlFontSize}
					/>
				</div>
			</Menu>
		</div>
	);
}

export default AdjustFontSize;
