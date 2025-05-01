import { useContext } from 'react';
import FuseLayoutSettingsContext from './FuseLayoutSettingsContext';

const useFuseLayoutSettings = () => {
	const context = useContext(FuseLayoutSettingsContext);

	if (context === undefined) {
		throw new Error('useFuseLayoutSettings must be used within a SettingsProvider');
	}

	return context;
};

export default useFuseLayoutSettings;
