'use client';
import { styled } from '@mui/material/styles';
import { Box, BoxProps } from '@mui/material';
import Icon from '@mui/material/Icon';
import clsx from 'clsx';

type FuseSvgIconProps = BoxProps & {
	fill?: string;
	xmlns?: string;
	viewBox?: string;
	size?: number | string;
	color?: 'inherit' | 'disabled' | 'primary' | 'secondary' | 'action' | 'error' | 'info' | 'success' | 'warning';
	ref?: React.RefObject<SVGSVGElement>;
};

/**
 * The Root styled component is used to style the root div of the FuseSvgIcon component.
 * It uses the styled function from the MUI styles library to create a styled component.
 */
const Root = styled(Box)<FuseSvgIconProps>(({ theme, size = 20, color = 'inherit' }) => ({
	width: size,
	height: size,
	minWidth: size,
	minHeight: size,
	fontSize: size,
	lineHeight: size,
	color: {
		primary: theme.vars.palette.primary.main,
		secondary: theme.vars.palette.secondary.main,
		info: theme.vars.palette.info.main,
		success: theme.vars.palette.success.main,
		warning: theme.vars.palette.warning.main,
		action: theme.vars.palette.action.active,
		error: theme.vars.palette.error.main,
		disabled: theme.vars.palette.action.disabled,
		inherit: 'currentColor'
	}[color] as string
}));

/**
 * The FuseSvgIcon component is responsible for rendering an SVG icon with a specified size and color.
 * It uses various MUI components to render the icon.
 * The component is memoized to prevent unnecessary re-renders.
 */
function FuseSvgIcon(props: FuseSvgIconProps) {
	const { children, className = '', color = 'inherit', ref } = props;

	if (typeof children !== 'string') {
		return null;
	}

	if (!children.includes(':')) {
		return (
			<Box
				component={Icon}
				ref={ref}
				{...props}
			/>
		);
	}

	const iconPath = children.replace(':', '.svg#');

	return (
		<Root
			{...props}
			as="svg"
			fill="none"
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 100 100"
			className={clsx('shrink-0 fill-current', className)}
			ref={ref}
			color={color}
		>
			<use xlinkHref={`/assets/icons/${iconPath}`} />
		</Root>
	);
}

export default FuseSvgIcon;
