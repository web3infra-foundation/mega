import { useTheme } from '@mui/material/styles';
import clsx from 'clsx';
import Typography from '@mui/material/Typography';
import { FuseThemeType } from '@fuse/core/FuseSettings/FuseSettings';

type SchemePreviewProps = {
	id: string;
	className?: string;
	onSelect: (T: FuseThemeType) => void;
	theme: FuseThemeType;
};

/**
 * The SchemePreview component is responsible for rendering a preview of a theme scheme.
 * It uses various MUI components to render the preview.
 * The component is memoized to prevent unnecessary re-renders.
 */
function SchemePreview(props: SchemePreviewProps) {
	const { theme, className, id, onSelect = () => {} } = props;

	const _theme = useTheme();

	const primaryColor: string = theme.palette.primary[500] ? theme.palette.primary[500] : theme.palette.primary.main;
	const primaryColorContrast = theme.palette.primary.contrastText || _theme.palette.getContrastText(primaryColor);

	const secondaryColor: string = theme.palette.secondary[500]
		? theme.palette.secondary[500]
		: theme.palette.secondary.main;
	const secondaryColorContrast =
		theme.palette.secondary.contrastText || _theme.palette.getContrastText(secondaryColor);
	const backgroundColor = theme.palette.background.default;
	const backgroundColorContrast = _theme.palette.getContrastText(theme.palette.background.default);
	const paperColor = theme.palette.background.paper;
	const paperColorContrast = _theme.palette.getContrastText(theme.palette.background.paper);

	return (
		<div className={clsx(className, 'mb-2')}>
			<button
				className={clsx(
					'relative w-full cursor-pointer overflow-hidden rounded-md text-left font-medium shadow-sm transition-shadow hover:shadow-md'
				)}
				style={{
					backgroundColor,
					color: backgroundColorContrast
				}}
				onClick={() => onSelect(theme)}
				type="button"
			>
				<div
					className="relative h-14 w-full px-2 pt-2"
					style={{
						backgroundColor: primaryColor,
						color: primaryColorContrast
					}}
				>
					<span className="text-md opacity-75">Header (Primary)</span>

					<div
						className="absolute bottom-0 right-0 -mb-2.5 mr-1 flex h-5 w-5 items-center justify-center rounded-full text-xs shadow-sm"
						style={{
							backgroundColor: secondaryColor,
							color: secondaryColorContrast
						}}
					>
						<span className="opacity-75">S</span>
					</div>
				</div>
				<div className="-mt-6 w-full pl-2 pr-7">
					<div
						className="relative h-24 w-full rounded-sm p-2 shadow-sm"
						style={{
							backgroundColor: paperColor,
							color: paperColorContrast
						}}
					>
						<span className="text-md opacity-75">Paper</span>
					</div>
				</div>

				<div className="w-full p-2">
					<span className="text-md opacity-75">Background</span>
				</div>
			</button>
			<Typography className="mt-3 w-full text-center font-semibold">{id}</Typography>
		</div>
	);
}

export default SchemePreview;
