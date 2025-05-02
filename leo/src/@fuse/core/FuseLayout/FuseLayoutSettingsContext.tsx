import { createContext } from 'react';
import { FuseSettingsConfigType } from '@fuse/core/FuseSettings/FuseSettings';

type FuseLayoutSettingsContextType = FuseSettingsConfigType['layout'];

const FuseLayoutSettingsContext = createContext<FuseLayoutSettingsContextType | undefined>(undefined);

export default FuseLayoutSettingsContext;
