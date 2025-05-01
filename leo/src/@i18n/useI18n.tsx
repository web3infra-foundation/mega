import { useContext } from 'react';
import I18nContext from './I18nContext';
import { I18nContextType } from './I18nContext';

const useI18n = () => {
	const context = useContext<I18nContextType>(I18nContext);

	if (!context) {
		throw new Error('useI18n must be used within an I18nProvider');
	}

	return context;
};

export default useI18n;
