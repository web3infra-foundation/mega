import { createSlice, PayloadAction, WithSlice } from '@reduxjs/toolkit';
import { ReactElement } from 'react';
import rootReducer from '@/store/rootReducer';

type InitialStateProps = {
	open: boolean;
	children: ReactElement | string;
};

/**
 * The initial state of the dialog slice.
 */
const initialState: InitialStateProps = {
	open: false,
	children: ''
};

/**
 * The Fuse Dialog slice
 */
export const fuseDialogSlice = createSlice({
	name: 'fuseDialog',
	initialState,
	reducers: {
		openDialog: (state, action: PayloadAction<{ children: InitialStateProps['children'] }>) => {
			state.open = true;
			state.children = action.payload.children;
		},
		closeDialog: () => initialState
	},
	selectors: {
		selectFuseDialogState: (fuseDialog) => fuseDialog.open,
		selectFuseDialogProps: (fuseDialog) => fuseDialog
	}
});

/**
 * Lazy load
 * */
rootReducer.inject(fuseDialogSlice);
const injectedSlice = fuseDialogSlice.injectInto(rootReducer);
declare module '@/store/rootReducer' {
	export interface LazyLoadedSlices extends WithSlice<typeof fuseDialogSlice> {}
}

export const { closeDialog, openDialog } = fuseDialogSlice.actions;

export const { selectFuseDialogState, selectFuseDialogProps } = injectedSlice.selectors;

export type dialogSliceType = typeof fuseDialogSlice;

export default fuseDialogSlice.reducer;
