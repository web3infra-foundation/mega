import { createSlice, PayloadAction, WithSlice } from '@reduxjs/toolkit';
import { ReactElement } from 'react';
import rootReducer from '@/store/rootReducer';

/**
 * The type definition for the initial state of the message slice.
 */
type initialStateProps = {
	state: boolean;
	options: {
		variant: 'success' | 'error' | 'warning' | 'info';
		anchorOrigin: {
			vertical: 'top' | 'bottom';
			horizontal: 'left' | 'center' | 'right';
		};
		autoHideDuration: number | null;
		message: ReactElement | string;
	};
};

/**
 * The initial state of the message slice.
 */
const initialState: initialStateProps = {
	state: false,
	options: {
		variant: 'info',
		anchorOrigin: {
			vertical: 'top',
			horizontal: 'center'
		},
		autoHideDuration: 2000,
		message: 'Hi'
	}
};

/**
 * The Message slice.
 */
export const fuseMessageSlice = createSlice({
	name: 'fuseMessage',
	initialState,
	reducers: {
		showMessage(state, action: PayloadAction<Partial<initialStateProps['options']>>) {
			state.state = true;
			state.options = {
				...initialState.options,
				...action.payload
			};
		},
		hideMessage(state) {
			state.state = false;
		}
	},
	selectors: {
		selectFuseMessageState: (fuseMessage) => fuseMessage.state,
		selectFuseMessageOptions: (fuseMessage) => fuseMessage.options
	}
});

/**
 * Lazy load
 * */
rootReducer.inject(fuseMessageSlice);
const injectedSlice = fuseMessageSlice.injectInto(rootReducer);
declare module '@/store/rootReducer' {
	export interface LazyLoadedSlices extends WithSlice<typeof fuseMessageSlice> {}
}

export const { hideMessage, showMessage } = fuseMessageSlice.actions;

export const { selectFuseMessageOptions, selectFuseMessageState } = injectedSlice.selectors;

export type messageSliceType = typeof fuseMessageSlice;

export default fuseMessageSlice.reducer;
