import clsx from 'clsx';
import Box from '@mui/material/Box';
import { Palette } from '@mui/material/styles';
import { PartialDeep } from 'type-fest';

/**
 * Props for PalettePreview component
 */
type PalettePreviewProps = {
	className?: string;
	palette: PartialDeep<Palette>;
};

/**
 * PalettePreview component
 */
function PalettePreview(props: PalettePreviewProps) {
	const { palette, className } = props;

	return (
		<Box
			className={clsx('relative w-50 overflow-hidden rounded-md text-left font-bold shadow-sm', className)}
			sx={{
				backgroundColor: palette.background.default,
				color: palette.text.primary
			}}
			type="button"
			component="button"
		>
			<Box
				className="relative h-14 w-full px-2 pt-2"
				sx={{
					backgroundColor: palette.primary.main,
					color: () => palette.primary.contrastText || palette.getContrastText(palette.primary.main)
				}}
			>
				<span className="text-md">Header (Primary)</span>

				<Box
					className="absolute bottom-0 right-0 -mb-2.5 mr-1 flex h-5 w-5 items-center justify-center rounded-full text-xs shadow-sm"
					sx={{
						backgroundColor: palette.secondary.main,
						color: () => palette.secondary.contrastText || palette.getContrastText(palette.secondary.main)
					}}
				>
					<span>S</span>
				</Box>
			</Box>
			<div className="-mt-6 w-full pl-2 pr-7">
				<Box
					className="relative h-24 w-full rounded-sm p-2 shadow-sm"
					sx={{
						backgroundColor: palette.background.paper,
						color: palette.text.primary
					}}
				>
					<span className="text-md opacity-75">Paper</span>
				</Box>
			</div>

			<div className="w-full p-2">
				<span className="text-md opacity-75">Background</span>
			</div>

			{/* <pre className="language-js p-6 w-100">{JSON.stringify(palette, null, 2)}</pre> */}
		</Box>
	);
}

export default PalettePreview;
