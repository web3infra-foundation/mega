import _ from 'lodash';
import * as colors from '@mui/material/colors';
import { FuseSettingsConfigType } from '@fuse/core/FuseSettings/FuseSettings';
import { User } from '@auth/user';
import { DeepPartial } from 'react-hook-form';
import { PartialDeep } from 'type-fest';
import EventEmitter from './EventEmitter';

type TreeNode = {
	id: string;
	children?: TreeNode[];
};
/**
 * The FuseRouteItemType type
 */
export type FuseRouteItemType = {
	path?: string;
	element?: React.ReactNode;
	auth?: string[] | [];
	settings?: DeepPartial<FuseSettingsConfigType>;
	children?: FuseRouteItemType[];
};

/**
 * The FuseRoutesType type is a custom type that is an array of FuseRouteItemType objects.
 */
export type FuseRoutesType = FuseRouteItemType[];

/**
 * The FuseRouteConfigType type is a custom type that defines the configuration for a set of routes.
 * It includes an optional routes property, an optional settings property, and an optional auth property.
 */
export type FuseRouteConfigType = {
	routes: FuseRoutesType;
	settings?: PartialDeep<FuseSettingsConfigType>;
	auth?: string[] | [];
};

/**
 * The FuseRouteConfigsType type is a custom type that is an array of FuseRouteConfigType objects.
 */
export type FuseRouteConfigsType = FuseRouteConfigType[] | [];

/**
 * The hueTypes type is a custom type that defines the possible values for a hue.
 */
type hueTypes =
	| '50'
	| '100'
	| '200'
	| '300'
	| '400'
	| '500'
	| '600'
	| '700'
	| '800'
	| '900'
	| 'A100'
	| 'A200'
	| 'A400'
	| 'A700';

type Color = {
	50?: string;
	100?: string;
	200?: string;
	300?: string;
	400?: string;
	500?: string;
	600?: string;
	700?: string;
	800?: string;
	900?: string;
	A100?: string;
	A200?: string;
	A400?: string;
	A700?: string;
	[key: string]: string | undefined;
};

/**
 * The FuseUtils class provides utility functions for the Fuse project.
 */
class FuseUtils {
	/**
	 * The filterArrayByString function filters an array of objects by a search string.
	 * It takes in an array of objects and a search string as parameters and returns a filtered array of objects.
	 *
	 */

	static filterArrayByString<T>(mainArr: T[], searchText: string): T[] {
		if (!searchText || searchText?.length === 0 || !searchText) {
			return mainArr; // Return the original array
		}

		searchText = searchText?.toLowerCase();
		const filtered = mainArr.filter((itemObj) => this.searchInObj(itemObj, searchText));

		if (filtered.length === mainArr.length) {
			return mainArr; // If the filtered array is identical, return the original
		}

		return filtered;
	}

	static filterArrayByString2<T>(mainArr: T[], searchText: string): T[] {
		if (typeof searchText !== 'string' || searchText === '') {
			return mainArr;
		}

		searchText = searchText?.toLowerCase();

		return mainArr.filter((itemObj: unknown) => this.searchInObj(itemObj, searchText));
	}

	/**
	 * The searchInObj function searches an object for a given search string.
	 * It takes in an object and a search string as parameters and returns a boolean indicating whether the search string was found in the object.
	 *
	 */
	static searchInObj(itemObj: unknown, searchText: string) {
		if (!isRecord(itemObj)) {
			return false;
		}

		const propArray = Object.keys(itemObj);

		function isRecord(value: unknown): value is Record<string, unknown> {
			return Boolean(value && typeof value === 'object' && !Array.isArray(value) && typeof value !== 'function');
		}

		for (const prop of propArray) {
			const value = itemObj[prop];

			if (typeof value === 'string') {
				if (this.searchInString(value, searchText)) {
					return true;
				}
			} else if (Array.isArray(value)) {
				if (this.searchInArray(value, searchText)) {
					return true;
				}
			}

			if (typeof value === 'object') {
				if (this.searchInObj(value, searchText)) {
					return true;
				}
			}
		}
		return false;
	}

	/**
	 * The searchInArray function searches an array for a given search string.
	 * It takes in an array and a search string as parameters and returns a boolean indicating whether the search string was found in the array.
	 *
	 */
	static searchInArray(arr: unknown[], searchText: string) {
		arr.forEach((value) => {
			if (typeof value === 'string') {
				if (this.searchInString(value, searchText)) {
					return true;
				}
			}

			if (value && typeof value === 'object') {
				if (this.searchInObj(value, searchText)) {
					return true;
				}
			}

			return false;
		});
		return false;
	}

	/**
	 * The searchInString function searches a string for a given search string.
	 * It takes in a string and a search string as parameters and returns a boolean indicating whether the search string was found in the string.
	 *
	 */
	static searchInString(value: string, searchText: string) {
		return value.toLowerCase().includes(searchText);
	}

	/**
	 * The generateGUID function generates a globally unique identifier.
	 * It returns a string representing the GUID.
	 *
	 */
	static generateGUID(): string {
		function S4() {
			return Math.floor((1 + Math.random()) * 0x10000)
				.toString(16)
				.substring(1);
		}

		return S4() + S4();
	}

	/**
	 * The toggleInArray function toggles an item in an array.
	 */
	static toggleInArray(item: unknown, array: unknown[]) {
		if (array.indexOf(item) === -1) {
			array.push(item);
		} else {
			array.splice(array.indexOf(item), 1);
		}
	}

	/**
	 * The handleize function converts a string to a handle.
	 */
	static handleize(text: string) {
		return text
			.toString()
			.toLowerCase()
			.replace(/\s+/g, '-') // Replace spaces with -
			.replace(/\W+/g, '') // Remove all non-word chars
			.replace(/--+/g, '-') // Replace multiple - with single -
			.replace(/^-+/, '') // Trim - from start of text
			.replace(/-+$/, ''); // Trim - from end of text
	}

	/**
	 * The setRoutes function sets the routes for the Fuse project.
	 */
	static setRoutes(
		config: FuseRouteConfigType,
		defaultAuth: FuseSettingsConfigType['defaultAuth'] = undefined
	): FuseRouteItemType[] {
		let routes: FuseRouteItemType[] = [];

		if (config?.routes) {
			routes = [...config.routes];
		}

		const applyAuth = (route: FuseRouteItemType, parentAuth: string[] | null) => {
			const auth = route.auth || route.auth === null ? route.auth : parentAuth;
			const settings = _.merge({}, config.settings, route.settings);

			const newRoute = {
				...route,
				settings,
				auth
			};

			if (route.children) {
				newRoute.children = route.children.map((childRoute) => applyAuth(childRoute, auth));
			}

			return newRoute;
		};

		routes = routes.map((route) => {
			const auth = config.auth || config.auth === null ? config.auth : defaultAuth || null;
			return applyAuth(route, auth);
		}) as FuseRouteItemType[];

		return [...routes];
	}

	/**
	 * The generateRoutesFromConfigs function generates routes from a set of route configurations.
	 * It takes in an array of route configurations as a parameter and returns an array of routes.
	 *
	 */
	static generateRoutesFromConfigs(
		configs: FuseRouteConfigsType,
		defaultAuth: FuseSettingsConfigType['defaultAuth']
	) {
		let allRoutes: FuseRouteItemType[] = [];
		configs.forEach((config: FuseRouteConfigType) => {
			allRoutes = [...allRoutes, ...this.setRoutes(config, defaultAuth)];
		});
		return allRoutes;
	}

	/**
	 * The findById function finds an object by its id.
	 */
	static findById(tree: TreeNode[], idToFind: string): TreeNode | undefined {
		// Try to find the node at the current level
		const node = _.find(tree, { id: idToFind });

		if (node) {
			return node;
		}

		let foundNode: TreeNode | undefined;

		// If not found, search in the children using lodash's some for iteration
		_.some(tree, (item) => {
			if (item.children) {
				foundNode = this.findById(item.children, idToFind);
				return foundNode; // If foundNode is truthy, _.some will stop iterating
			}

			return false; // Continue iterating
		});

		return foundNode;
	}

	/**
	 * The randomMatColor function generates a random material color.
	 */
	static randomMatColor(hue: hueTypes = '400') {
		const mainColors = [
			'red',
			'pink',
			'purple',
			'deepPurple',
			'indigo',
			'blue',
			'lightBlue',
			'cyan',
			'teal',
			'green',
			'lightGreen',
			'lime',
			'yellow',
			'amber',
			'orange',
			'deepOrange'
		];

		const randomColor = mainColors[Math.floor(Math.random() * mainColors.length)];

		return (colors as Record<string, Color>)[randomColor][hue];
	}

	/**
	 * The findNavItemById function finds a navigation item by its id.
	 */
	static difference(object: Record<string, unknown>, base: Record<string, unknown>): Record<string, unknown> {
		function changes(_object: Record<string, unknown>, _base: Record<string, unknown>): Record<string, unknown> {
			return _.transform(
				_object,
				(result: Record<string, unknown>, value: unknown, key: string) => {
					if (!_.isEqual(value, _base[key])) {
						result[key] =
							_.isObject(value) && _.isObject(_base[key])
								? changes(value as Record<string, unknown>, _base[key] as Record<string, unknown>)
								: value;
					}
				},
				{}
			);
		}

		return changes(object, base);
	}

	/**
	 * The EventEmitter class is a custom implementation of an event emitter.
	 * It provides methods for registering and emitting events.
	 */
	static EventEmitter = EventEmitter;

	/**
	 * The hasPermission function checks if a user has permission to access a resource.
	 */
	static hasPermission(authArr: string[] | string | undefined, userRole: User['role']): boolean {
		/**
		 * If auth array is not defined
		 * Pass and allow
		 */
		if (authArr === null || authArr === undefined) {
			return true;
		}

		if (Array.isArray(authArr) && authArr?.length === 0) {
			/**
			 * if auth array is empty means,
			 * allow only user role is guest (null or empty[])
			 */
			return !userRole || userRole.length === 0;
		}

		/**
		 * Check if user has grants
		 */
		/*
            Check if user role is array,
            */
		if (userRole && Array.isArray(authArr) && Array.isArray(userRole)) {
			return authArr.some((r: string) => userRole.indexOf(r) >= 0);
		}

		if (typeof userRole === 'string' && Array.isArray(authArr)) {
			return authArr?.includes?.(userRole);
		}

		return false;
	}

	/**
	 * The filterArrayByString function filters an array of objects by a search string.
	 */
	static filterRecursive(data: [] | null, predicate: (arg0: unknown) => boolean) {
		// if no data is sent in, return null, otherwise transform the data
		return !data
			? null
			: data.reduce((list: unknown[], entry: { children?: [] }) => {
					let clone: unknown = null;

					if (predicate(entry)) {
						// if the object matches the filter, clone it as it is
						clone = { ...entry };
					}

					if (entry.children != null) {
						// if the object has childrens, filter the list of children
						const children = this.filterRecursive(entry.children, predicate);

						if (children && children?.length > 0) {
							// if any of the children matches, clone the parent object, overwrite
							// the children list with the filtered list
							clone = { ...entry, children };
						}
					}

					// if there's a cloned object, push it to the output list
					if (clone) {
						list.push(clone);
					}

					return list;
				}, []);
	}
}

export default FuseUtils;
