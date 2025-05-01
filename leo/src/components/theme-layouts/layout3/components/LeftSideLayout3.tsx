import FuseSidePanel from '@fuse/core/FuseSidePanel';
import { memo } from 'react';
import NavigationShortcuts from '../../components/navigation/NavigationShortcuts';

/**
 * The left side layout 3.
 */
function LeftSideLayout3() {
	return (
		<FuseSidePanel className="flex flex-col justify-star items-center py-3 space-y-2">
			<div>
				<NavigationShortcuts
					className="flex shrink"
					variant="vertical"
				/>
			</div>
		</FuseSidePanel>
	);
}

export default memo(LeftSideLayout3);
