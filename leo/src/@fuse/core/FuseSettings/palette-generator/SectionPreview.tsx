import clsx from 'clsx';
import Box from '@mui/material/Box';
import { lighten } from '@mui/material/styles';

/**
 * Props for SectionPreview component
 */
type SectionPreviewProps = {
	className?: string;
	section?: 'main' | 'navbar' | 'toolbar' | 'footer';
};

/**
 * SectionPreview component
 */
function SectionPreview(props: SectionPreviewProps) {
	const { section, className } = props;
	return (
		<div className={clsx('flex h-20 w-32 overflow-hidden rounded-md border-1 hover:opacity-80', className)}>
			<Box
				sx={[
					section === 'navbar'
						? {
								backgroundColor: (theme) => `rgba(${theme.vars.palette.secondary.mainChannel} / 0.3)`
							}
						: {
								backgroundColor: (theme) =>
									lighten(
										theme.palette.background.default,
										theme.palette.mode === 'light' ? 0.4 : 0.02
									)
							},
					section === 'navbar'
						? {
								'& > div': {
									backgroundColor: (theme) =>
										`rgba(${theme.vars.palette.secondary.mainChannel} / 0.3)`
								}
							}
						: {
								'& > div': {
									backgroundColor: (theme) => theme.vars.palette.divider
								}
							}
				]}
				className="w-8 space-y-0.25 px-1.5 pt-3"
			>
				<div className="h-1 rounded-xs" />
				<div className="h-1 rounded-xs" />
				<div className="h-1 rounded-xs" />
				<div className="h-1 rounded-xs" />
				<div className="h-1 rounded-xs" />
			</Box>
			<div className="flex flex-auto flex-col border-l">
				<Box
					sx={[
						section === 'toolbar'
							? {
									backgroundColor: (theme) =>
										`rgba(${theme.vars.palette.secondary.mainChannel} / 0.3)`
								}
							: {
									backgroundColor: (theme) =>
										lighten(
											theme.palette.background.default,
											theme.palette.mode === 'light' ? 0.4 : 0.02
										)
								},
						section === 'toolbar'
							? {
									'& > div': {
										backgroundColor: (theme) =>
											`rgba(${theme.vars.palette.secondary.mainChannel} / 0.3)`
									}
								}
							: {
									'& > div': {
										backgroundColor: (theme) => theme.vars.palette.divider
									}
								}
					]}
					className={clsx('flex h-3 items-center justify-end pr-1.5')}
				>
					<div className="ml-1 h-1 w-1 rounded-full" />
					<div className="ml-1 h-1 w-1 rounded-full" />
					<div className="ml-1 h-1 w-1 rounded-full" />
				</Box>
				<Box
					sx={[
						section === 'main'
							? {
									backgroundColor: (theme) =>
										`rgba(${theme.vars.palette.secondary.mainChannel} / 0.3)`
								}
							: {
									backgroundColor: (theme) =>
										lighten(
											theme.palette.background.default,
											theme.palette.mode === 'light' ? 0.4 : 0.02
										)
								}
					]}
					className={clsx('flex flex-auto border-y')}
				/>
				<Box
					sx={[
						section === 'footer'
							? {
									backgroundColor: (theme) =>
										`rgba(${theme.vars.palette.secondary.mainChannel} / 0.3)`
								}
							: {
									backgroundColor: (theme) =>
										lighten(
											theme.palette.background.default,
											theme.palette.mode === 'light' ? 0.4 : 0.02
										)
								},
						section === 'footer'
							? {
									'& > div': {
										backgroundColor: (theme) => `rgba(${theme.palette.secondary.mainChannel} / 0.3)`
									}
								}
							: {
									'& > div': {
										backgroundColor: (theme) => theme.vars.palette.divider
									}
								}
					]}
					className={clsx('flex h-3 items-center pr-1.5')}
				>
					<div className="ml-1 h-1 w-1 rounded-full" />
					<div className="ml-1 h-1 w-1 rounded-full" />
					<div className="ml-1 h-1 w-1 rounded-full" />
				</Box>
			</div>
		</div>
	);
}

export default SectionPreview;
