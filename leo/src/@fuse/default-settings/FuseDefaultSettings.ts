'use client';

import { fuseDark } from '@fuse/colors';
import { lightBlue, red } from '@mui/material/colors';
import { createTheme, ThemeOptions } from '@mui/material/styles';
import qs from 'qs';
import { FuseSettingsConfigType } from '@fuse/core/FuseSettings/FuseSettings';
import type {} from '@mui/material/themeCssVarsAugmentation';

/**
 * The defaultTheme object defines the default color palette for the application.
 */
const defaultTheme = {
	palette: {
		mode: 'light',
		text: {
			primary: 'rgb(17, 24, 39)',
			secondary: 'rgb(107, 114, 128)',
			disabled: 'rgb(149, 156, 169)'
		},
		common: {
			black: 'rgb(17, 24, 39)',
			white: 'rgb(255, 255, 255)'
		},
		primary: {
			light: '#bec1c5',
			main: '#252f3e',
			dark: '#0d121b',
			contrastDefaultColor: 'light'
		},
		secondary: {
			light: '#bdf2fa',
			main: '#22d3ee',
			dark: '#0cb7e2'
		},
		background: {
			paper: '#FFFFFF',
			default: '#f6f7f9'
		},
		error: {
			light: '#ffcdd2',
			main: '#f44336',
			dark: '#b71c1c'
		}
	}
};

/**
 * The defaultSettings object defines the default settings for the Fuse application.
 */
export const defaultSettings = {
	customScrollbars: true,
	direction: 'ltr',
	layout: {},
	theme: {
		main: defaultTheme,
		navbar: defaultTheme,
		toolbar: defaultTheme,
		footer: defaultTheme
	}
};

/**
 * The getParsedQuerySettings function parses the query string to retrieve the default settings for the Fuse application.
 * It returns a FuseSettingsConfigType object that can be used to configure the application.
 */
export function getParsedQuerySettings(): FuseSettingsConfigType | object {
	if (typeof window === 'undefined') {
		return null;
	}

	const parsedQueryString = qs.parse(window?.location?.search, { ignoreQueryPrefix: true });

	const { defaultSettings = {} } = parsedQueryString;

	if (typeof defaultSettings === 'string') {
		// Handle the case when defaultSettings is a string
		return JSON.parse(defaultSettings) as FuseSettingsConfigType;
	}

	return {};

	// Generating route params from settings
	/* const settings = qs.stringify({
        defaultSettings: JSON.stringify(defaultSettings, {strictNullHandling: true})
    });
    console.info(settings); */
}

/**
 * The defaultThemeOptions object defines the default options for the MUI theme.
 */
export const defaultThemeOptions = {
	cssVariables: true,
	typography: {
		fontFamily: ['Inter var', 'Roboto', '"Helvetica"', 'Arial', 'sans-serif'].join(','),
		fontWeightLight: 300,
		fontWeightRegular: 400,
		fontWeightMedium: 500
	},
	breakpoints: {
		values: {
			xs: 0, // Extra small devices
			sm: 600, // Small devices
			md: 960, // Medium devices
			lg: 1280, // Large devices
			xl: 1920 // Extra large devices
		}
	},
	components: {
		MuiSvgIcon: {
			defaultProps: {},
			styleOverrides: {
				root: {},
				sizeSmall: {
					width: 16,
					height: 16
				},
				sizeMedium: {
					width: 20,
					height: 20
				},
				sizeLarge: {
					width: 24,
					height: 24
				}
			}
		},
		MuiAppBar: {
			defaultProps: {
				enableColorOnDark: true
			},
			styleOverrides: {
				root: {
					backgroundImage: 'none'
				}
			}
		},
		MuiPickersPopper: {
			styleOverrides: {
				root: {
					zIndex: 99999
				}
			}
		},
		MuiAutocomplete: {
			styleOverrides: {
				popper: {
					zIndex: 99999
				}
			}
		},
		MuiButtonBase: {
			defaultProps: {
				// disableRipple: true
			},
			styleOverrides: {
				root: {}
			}
		},
		MuiIconButton: {
			styleOverrides: {
				root: {
					borderRadius: 8
				},
				sizeMedium: {
					width: 36,
					height: 36,
					maxHeight: 36
				},
				sizeSmall: {
					width: 32,
					height: 32,
					maxHeight: 32
				},
				sizeLarge: {
					width: 40,
					height: 40,
					maxHeight: 40
				}
			}
		},
		MuiBadge: {
			defaultProps: {},
			styleOverrides: {
				root: {}
			}
		},
		MuiAvatar: {
			defaultProps: {},
			styleOverrides: {
				root: {
					width: 36,
					height: 36
				}
			}
		},
		MuiButton: {
			defaultProps: {
				variant: 'text',
				color: 'inherit'
			},
			styleOverrides: {
				root: {
					textTransform: 'none'
					// lineHeight: 1,
				},
				sizeMedium: {
					borderRadius: 8,
					height: 36,
					minHeight: 36,
					maxHeight: 36
				},
				sizeSmall: {
					borderRadius: 8,
					height: 32,
					minHeight: 32,
					maxHeight: 32
				},
				sizeLarge: {
					height: 40,
					minHeight: 40,
					maxHeight: 40,
					borderRadius: 8
				},
				contained: {
					boxShadow: 'none',
					'&:hover, &:focus': {
						boxShadow: 'none'
					}
				}
			}
		},
		MuiButtonGroup: {
			defaultProps: {
				color: 'secondary'
			},
			styleOverrides: {
				contained: {
					borderRadius: 8
				}
			}
		},
		MuiTab: {
			styleOverrides: {
				root: {
					textTransform: 'none'
				}
			}
		},
		MuiDrawer: {
			styleOverrides: {
				paper: {}
			}
		},
		MuiDialog: {
			styleOverrides: {
				paper: {
					borderRadius: 12
				}
			}
		},
		MuiPaper: {
			styleOverrides: {
				root: {
					backgroundImage: 'none'
				},
				rounded: {
					borderRadius: 12
				}
			}
		},
		MuiCard: {
			styleOverrides: {}
		},
		MuiPopover: {
			styleOverrides: {
				paper: {
					borderRadius: 8
				}
			}
		},
		MuiTextField: {
			defaultProps: {
				color: 'secondary'
			},
			styleOverrides: {
				root: {
					'& > .MuiFormHelperText-root': {
						marginLeft: 11
					}
				}
			}
		},
		MuiInputLabel: {
			defaultProps: {
				color: 'secondary'
			},
			styleOverrides: {
				shrink: {
					transform: 'translate(11px, -7px) scale(0.8)'
				},
				root: {
					transform: 'translate(11px, 8px) scale(1)',
					'&.Mui-focused': {}
				}
			}
		},
		MuiSelect: {
			defaultProps: {
				color: 'secondary'
			},
			styleOverrides: {
				select: {
					minHeight: 0
				}
			}
		},
		MuiFormHelperText: {
			styleOverrides: {
				root: {}
			}
		},
		MuiInputAdornment: {
			styleOverrides: {
				root: {
					marginRight: 0
				}
			}
		},
		MuiInputBase: {
			styleOverrides: {
				root: {
					// height: 36,
					minHeight: 36,
					borderRadius: 8,
					lineHeight: 1
				},
				legend: {
					fontSize: '0.75em'
				},
				input: {
					padding: '5px 11px'
				},
				adornedStart: {
					paddingLeft: `11px!important`
				},
				sizeSmall: {
					height: 32,
					minHeight: 32,
					borderRadius: 8
				},
				sizeMedium: {
					height: 36,
					minHeight: 36,
					borderRadius: 8
				},
				sizeLarge: {
					height: 40,
					minHeight: 40,
					borderRadius: 8
				}
			}
		},
		MuiOutlinedInput: {
			defaultProps: {
				color: 'secondary'
			},
			styleOverrides: {
				root: {
					// paddingLeft: 11
				},
				input: {
					padding: '5px 11px'
				}
			}
		},
		MuiFilledInput: {
			styleOverrides: {
				root: {
					borderRadius: 8,
					'&:before, &:after': {
						display: 'none'
					}
				},

				input: {
					padding: '5px 11px'
				}
			}
		},
		MuiSlider: {
			defaultProps: {
				color: 'secondary'
			}
		},
		MuiCheckbox: {
			defaultProps: {
				color: 'secondary'
			}
		},
		MuiRadio: {
			defaultProps: {
				color: 'secondary'
			}
		},
		MuiSwitch: {
			defaultProps: {
				color: 'secondary'
			}
		},
		MuiTypography: {
			variants: [
				{
					props: { color: 'text.secondary' },
					style: {
						color: 'text.secondary'
					}
				}
			]
		}
	}
};

/**
 * The mustHaveThemeOptions object defines the options that must be present in the MUI theme.
 */
export const mustHaveThemeOptions = {
	typography: {
		fontSize: 13,
		body1: {
			fontSize: '0.8125rem'
		},
		body2: {
			fontSize: '0.8125rem'
		}
	}
};

/**
 * The defaultThemes object defines the default themes for the application.
 */
export const defaultThemes = {
	default: {
		palette: {
			mode: 'light',
			primary: fuseDark,
			secondary: {
				light: lightBlue[400],
				main: lightBlue[600],
				dark: lightBlue[700]
			},
			error: red
		},
		status: {
			danger: 'orange'
		}
	},
	defaultDark: {
		palette: {
			mode: 'dark',
			primary: fuseDark,
			secondary: {
				light: lightBlue[400],
				main: lightBlue[600],
				dark: lightBlue[700]
			},
			error: red
		},
		status: {
			danger: 'orange'
		}
	}
};

/**
 * The extendThemeWithMixins function extends the theme with mixins.
 */
export function extendThemeWithMixins(obj: ThemeOptions) {
	const theme = createTheme(obj);
	return {
		border: (width = 1) => ({
			borderWidth: width,
			borderStyle: 'solid',
			borderColor: theme.vars.palette.divider
		}),
		borderLeft: (width = 1) => ({
			borderLeftWidth: width,
			borderStyle: 'solid',
			borderColor: theme.vars.palette.divider
		}),
		borderRight: (width = 1) => ({
			borderRightWidth: width,
			borderStyle: 'solid',
			borderColor: theme.vars.palette.divider
		}),
		borderTop: (width = 1) => ({
			borderTopWidth: width,
			borderStyle: 'solid',
			borderColor: theme.vars.palette.divider
		}),
		borderBottom: (width = 1) => ({
			borderBottomWidth: width,
			borderStyle: 'solid',
			borderColor: theme.vars.palette.divider
		})
	};
}
