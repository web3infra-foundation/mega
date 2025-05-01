import { createContext } from 'react';

export type LanguageType = {
	id: string;
	title: string;
	flag: string;
};

export type I18nContextType = {
	language: LanguageType;
	languageId: string;
	languages: LanguageType[];
	changeLanguage: (languageId: string) => Promise<void>;
	langDirection: 'ltr' | 'rtl';
};

const I18nContext = createContext<I18nContextType | undefined>(undefined);

export default I18nContext;
