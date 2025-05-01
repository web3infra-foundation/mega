'use client';

import FuseShortcuts from '@fuse/core/FuseShortcuts';
import { useEffect, useState } from 'react';
import useUser from '@auth/useUser';
import useNavigation from './hooks/useNavigation';

type NavigationShortcutsProps = {
	className?: string;
	variant?: 'horizontal' | 'vertical';
};

/**
 * The navigation shortcuts.
 */
function NavigationShortcuts(props: NavigationShortcutsProps) {
	const { variant, className } = props;
	const { flattenNavigation: navigation } = useNavigation();
	const { data: user, isGuest } = useUser();
	const [userShortcuts, setUserShortcuts] = useState<string[]>(user?.shortcuts || []);
	// const prevUserShortcuts = usePrevious(userShortcuts);

	useEffect(() => {
		/**
		 * Updating user settings disabled for demonstration purposes
		 * The request is made to the mock API and will not persist the changes
		 * You can enable it by removing the comment block below when using a real API
		 * */
		/* if (!isGuest && prevUserShortcuts && !_.isEqual(userShortcuts, prevUserShortcuts)) {
			updateUser(setIn(user, 'shortcuts', userShortcuts) as User);
		} */
	}, [isGuest, userShortcuts]);

	function handleShortcutsChange(newShortcuts: string[]) {
		setUserShortcuts(newShortcuts);
	}

	return (
		<FuseShortcuts
			className={className}
			variant={variant}
			navigation={navigation}
			shortcuts={userShortcuts}
			onChange={handleShortcutsChange}
		/>
	);
}

export default NavigationShortcuts;
