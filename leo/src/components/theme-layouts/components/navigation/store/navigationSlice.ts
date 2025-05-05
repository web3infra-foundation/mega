import { createEntityAdapter, createSlice, PayloadAction } from '@reduxjs/toolkit';
import { AppThunk, RootState } from 'src/store/store';
import { PartialDeep } from 'type-fest';
import { FuseFlatNavItemType, FuseNavItemType } from '@fuse/core/FuseNavigation/types/FuseNavItemType';
import FuseNavigationHelper from '@fuse/utils/FuseNavigationHelper';
import FuseNavItemModel from '@fuse/core/FuseNavigation/models/FuseNavItemModel';
import navigationConfig from 'src/configs/navigationConfig';

const navigationAdapter = createEntityAdapter<FuseFlatNavItemType>();

const emptyInitialState = navigationAdapter.getInitialState([]);

const initialState = navigationAdapter.upsertMany(
	emptyInitialState,
	FuseNavigationHelper.flattenNavigation(navigationConfig)
);

/**
 * Redux Thunk actions related to the navigation store state
 */
/**
 * Appends a navigation item to the navigation store state.
 */
export const appendNavigationItem =
	(item: FuseNavItemType, parentId?: string | null): AppThunk =>
	async (dispatch, getState) => {
		const AppState = getState();
		const navigation = FuseNavigationHelper.unflattenNavigation(selectNavigationAll(AppState));

		dispatch(setNavigation(FuseNavigationHelper.appendNavItem(navigation, FuseNavItemModel(item), parentId)));

		return Promise.resolve();
	};

/**
 * Prepends a navigation item to the navigation store state.
 */
export const prependNavigationItem =
	(item: FuseNavItemType, parentId?: string | null): AppThunk =>
	async (dispatch, getState) => {
		const AppState = getState();
		const navigation = FuseNavigationHelper.unflattenNavigation(selectNavigationAll(AppState));

		dispatch(setNavigation(FuseNavigationHelper.prependNavItem(navigation, FuseNavItemModel(item), parentId)));

		return Promise.resolve();
	};

/**
 * Adds a navigation item to the navigation store state at the specified index.
 */
export const updateNavigationItem =
	(id: string, item: PartialDeep<FuseNavItemType>): AppThunk =>
	async (dispatch, getState) => {
		const AppState = getState();
		const navigation = FuseNavigationHelper.unflattenNavigation(selectNavigationAll(AppState));

		dispatch(setNavigation(FuseNavigationHelper.updateNavItem(navigation, id, item)));

		return Promise.resolve();
	};

/**
 * Removes a navigation item from the navigation store state.
 */
export const removeNavigationItem =
	(id: string): AppThunk =>
	async (dispatch, getState) => {
		const AppState = getState();
		const navigation = FuseNavigationHelper.unflattenNavigation(selectNavigationAll(AppState));

		dispatch(setNavigation(FuseNavigationHelper.removeNavItem(navigation, id)));

		return Promise.resolve();
	};

export const {
	selectAll: selectNavigationAll,
	selectIds: selectNavigationIds,
	selectById: selectNavigationItemById
} = navigationAdapter.getSelectors<RootState>((state) => state.navigation);

/**
 * The navigation slice
 */
export const navigationSlice = createSlice({
	name: 'navigation',
	initialState,
	reducers: {
		setNavigation(state, action: PayloadAction<FuseNavItemType[]>) {
			return navigationAdapter.setAll(state, FuseNavigationHelper.flattenNavigation(action.payload));
		},
		resetNavigation: () => initialState
	}
});

export const { setNavigation, resetNavigation } = navigationSlice.actions;

export type navigationSliceType = typeof navigationSlice;

export default navigationSlice.reducer;
