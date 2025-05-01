import { FuseNavItemType } from './types/FuseNavItemType';
import components from './utils/components';

export type FuseNavItemComponentProps = {
	type: string;
	item: FuseNavItemType;
	dense?: boolean;
	nestedLevel?: number;
	onItemClick?: (T: FuseNavItemType) => void;
	checkPermission?: boolean;
};

/**
Component to render NavItem depending on its type.
*/
function FuseNavItem(props: FuseNavItemComponentProps) {
	const { type } = props;

	const C = components[type];

	return C ? <C {...(props as object)} /> : null;
}

export default FuseNavItem;
