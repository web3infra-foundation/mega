import { Reducer } from '@reduxjs/toolkit';
import { useEffect, useState } from 'react';
import rootReducer from './rootReducer';
/**
 * A Higher Order Component that injects a reducer into the Redux store.
 */
const withReducer =
	<P extends object>(key: string, reducer: Reducer) =>
	(WrappedComponent: React.FC<P>) => {
		rootReducer.inject(
			{
				reducerPath: key,
				reducer
			},
			{
				overrideExisting: true
			}
		);
		/**
		 * The component that wraps the provided component with the injected reducer.
		 */
		return function WithInjectedReducer(props: P) {
			const [awaitRender, setAwaitRender] = useState<boolean>(true);

			useEffect(() => {
				setTimeout(() => {
					setAwaitRender(false);
				}, 30000);
			}, []);
			return awaitRender ? null : <WrappedComponent {...props} />;
		};
	};
export default withReducer;
