import { createSlice } from '@reduxjs/toolkit';
import type { WithSlice } from '@reduxjs/toolkit';
import rootReducer from '@/store/rootReducer';

const exampleData = {
	notes: [
		{
			id: 1,
			title: 'Best songs to listen while working',
			detail: 'Last edit: May 8th, 2015'
		},
		{
			id: 2,
			title: 'Useful subreddits',
			detail: 'Last edit: January 12th, 2015'
		}
	],
	events: [
		{
			id: 1,
			title: 'Group Meeting',
			detail: 'In 32 Minutes, Room 1B'
		},
		{
			id: 2,

			title: 'Public Beta Release',
			detail: '11:00 PM'
		},
		{
			id: 3,
			title: 'Dinner with David',
			detail: '17:30 PM'
		},
		{
			id: 4,
			title: 'Q&A Session',
			detail: '20:30 PM'
		}
	]
};
/**
 * Quick Panel data slice.
 */
export const quickPanelSlice = createSlice({
	name: 'quickPanel',
	initialState: { data: exampleData, open: false },
	reducers: {
		removeEvents: (state) => {
			state.data.events = [];
		},
		toggleQuickPanel: (state) => {
			state.open = !state.open;
		},
		openQuickPanel: (state) => {
			state.open = true;
		},
		closeQuickPanel: (state) => {
			state.open = false;
		}
	},
	selectors: {
		selectQuickPanelData: (state) => state.data,
		selectQuickPanelOpen: (state) => state.open
	}
});

/**
 * Lazy loading
 */
rootReducer.inject(quickPanelSlice);
const injectedSlice = quickPanelSlice.injectInto(rootReducer);
declare module '@/store/rootReducer' {
	export interface LazyLoadedSlices extends WithSlice<typeof quickPanelSlice> {}
}

export const { selectQuickPanelData, selectQuickPanelOpen } = injectedSlice.selectors;

export const { removeEvents, toggleQuickPanel, openQuickPanel, closeQuickPanel } = quickPanelSlice.actions;

export type dataSliceType = typeof quickPanelSlice;

export default quickPanelSlice.reducer;
