import { useState, ReactNode, useMemo, useEffect, useCallback } from 'react';
import _ from 'lodash';
import { defaultSettings, getParsedQuerySettings } from '@fuse/default-settings';
import settingsConfig from 'src/configs/settingsConfig';
import themeLayoutConfigs from 'src/components/theme-layouts/themeLayoutConfigs';
import { FuseSettingsConfigType, FuseThemesType } from '@fuse/core/FuseSettings/FuseSettings';
import useUser from '@auth/useUser';
import { PartialDeep } from 'type-fest';
import FuseSettingsContext from './FuseSettingsContext';

// Get initial settings
const getInitialSettings = (): FuseSettingsConfigType => {
	const defaultLayoutStyle = settingsConfig.layout?.style || 'layout1';
	const layout = {
		style: defaultLayoutStyle,
		config: themeLayoutConfigs[defaultLayoutStyle]?.defaults
	};
	return _.merge({}, defaultSettings, { layout }, settingsConfig, getParsedQuerySettings());
};

const initialSettings = getInitialSettings();

const generateSettings = (
	_defaultSettings: FuseSettingsConfigType,
	_newSettings: PartialDeep<FuseSettingsConfigType>
) => {
	return _.merge(
		{},
		_defaultSettings,
		{ layout: { config: themeLayoutConfigs[_newSettings?.layout?.style]?.defaults } },
		_newSettings
	);
};

// FuseSettingsProvider component
export function FuseSettingsProvider({ children }: { children: ReactNode }) {
	const { data: user, isGuest } = useUser();

	const userSettings = useMemo(() => user?.settings || {}, [user]);

	const calculateSettings = useCallback(() => {
		const defaultSettings = _.merge({}, initialSettings);
		return isGuest ? defaultSettings : _.merge({}, defaultSettings, userSettings);
	}, [isGuest, userSettings]);

	const [data, setData] = useState<FuseSettingsConfigType>(calculateSettings());

	// Sync data with userSettings when isGuest or userSettings change
	useEffect(() => {
		const newSettings = calculateSettings();

		// Only update if settings are different
		if (!_.isEqual(data, newSettings)) {
			setData(newSettings);
		}
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [calculateSettings]);

	const setSettings = useCallback(
		(newSettings: Partial<FuseSettingsConfigType>) => {
			const _settings = generateSettings(data, newSettings);

			if (!_.isEqual(_settings, data)) {
				setData(_.merge({}, _settings));
			}

			return _settings;
		},
		[data]
	);

	const changeTheme = useCallback(
		(newTheme: FuseThemesType) => {
			const { navbar, footer, toolbar, main } = newTheme;

			const newSettings: FuseSettingsConfigType = {
				...data,
				theme: {
					main,
					navbar,
					toolbar,
					footer
				}
			};

			setSettings(newSettings);
		},
		[data, setSettings]
	);

	return (
		<FuseSettingsContext
			value={useMemo(
				() => ({
					data,
					setSettings,
					changeTheme
				}),
				[data, setSettings, changeTheme]
			)}
		>
			{children}
		</FuseSettingsContext>
	);
}

export default FuseSettingsProvider;
