import { FuseSettingsConfigType, FuseThemesType } from '@fuse/core/FuseSettings/FuseSettings';
import { createContext } from 'react';

// FuseSettingsContext type
export type FuseSettingsContextType = {
	data: FuseSettingsConfigType;
	setSettings: (newSettings: Partial<FuseSettingsConfigType>) => FuseSettingsConfigType;
	changeTheme: (newTheme: FuseThemesType) => void;
};

// Context with a default value of undefined
const FuseSettingsContext = createContext<FuseSettingsContextType | undefined>(undefined);

export default FuseSettingsContext;
