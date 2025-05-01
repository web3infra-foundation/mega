import { Pathname } from 'history';
import { FuseNavItemType } from './types/FuseNavItemType';

/**
 * Determines whether a given URL is present in the parent's child list or not.
 */
const isUrlInChildren = (parent: FuseNavItemType, url: Pathname) => {
	if (!parent.children) {
		return false;
	}

	for (const navItem of parent.children) {
		if (navItem.children) {
			if (isUrlInChildren(navItem, url)) {
				return true;
			}
		}

		if (navItem.url === url || url.includes(navItem.url)) {
			return true;
		}
	}

	return false;
};

export default isUrlInChildren;
