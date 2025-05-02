/* eslint-disable */
// eslint-disable-next-line import/no-extraneous-dependencies
import plugin from 'tailwindcss/plugin';

/**
 * The iconSize function is a Tailwind CSS plugin that generates utility classes for setting the size of icons.
 */
const iconSize = plugin(
	({ addUtilities, theme, matchUtilities }) => {
		const spacingScale = theme('spacing');

		const createIconStyles = (value) => ({
			width: value,
			height: value,
			minWidth: value,
			minHeight: value,
			fontSize: value,
			lineHeight: value,
			'svg': {
				width: value,
				height: value
			}
		});

		// Standard spacing scale utilities
		addUtilities(
			Object.entries(spacingScale).map(([key, value]) => ({
				[`.icon-size-${key}`]: createIconStyles(value)
			}))
		);

		// Arbitrary value support
		matchUtilities(
			{
				'icon-size': (value) => createIconStyles(value)
			},
			{ values: spacingScale }
		);
	}
);

export default iconSize;
