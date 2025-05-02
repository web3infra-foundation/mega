import { createContext } from 'react';

/**
 * The type of the AppContext value.
 */
export type AppContextType = Record<string, never>;

/**
 * The AppContext object.
 */
const AppContext = createContext<AppContextType>({});

export default AppContext;
