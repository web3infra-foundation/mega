import React from 'react';
import { Reducer, Slice } from '@reduxjs/toolkit';
import generateReducersFromSlices from './generateReducersFromSlices';
import rootReducer from './rootReducer';
import store from './store';

export type SlicesType = Slice[];

const injectedReducers = new Set<string>();

export const injectReducersGroupedByCommonKey = async (slices: SlicesType): Promise<void> => {
	const reducers = generateReducersFromSlices(slices);

	if (reducers) {
		Object.keys(reducers).forEach((key) => {
			// Only inject if it has not been injected yet
			if (!injectedReducers.has(key)) {
				const reducer = reducers[key] as Reducer;

				if (!key || !reducer) {
					return;
				}

				rootReducer.inject(
					{
						reducerPath: key,
						reducer
					},
					{
						overrideExisting: true
					}
				);

				// Add to the set of injected reducers
				injectedReducers.add(key);

				// Dispatch a dummy action to ensure the Redux store recognizes the new reducer
				store.dispatch({ type: `@@INIT/${key}` });
			}
		});
	}
};

/**
 * A Higher Order Component that injects reducers for the provided slices.
 */
const withSlices =
	<P extends object>(slices: SlicesType) =>
	(WrappedComponent: React.FC<P>) => {
		return function WithInjectedReducer(props: P) {
			const [isInjected, setIsInjected] = React.useState(false);

			React.useEffect(() => {
				const injectSlices = async () => {
					// Inject slices and dispatch an init action to "wake up" the reducers
					await injectReducersGroupedByCommonKey(slices);
					setIsInjected(true);
				};

				injectSlices();
			}, []);

			if (!isInjected) {
				return null; // Or a loading indicator
			}

			return <WrappedComponent {...props} />;
		};
	};

export default withSlices;
