import { useSession, signOut } from 'next-auth/react';
import { useMemo } from 'react';
import { User } from '@auth/user';
import { authUpdateDbUser } from '@auth/authApi';
import _ from 'lodash';
import setIn from '@/utils/setIn';

type useUser = {
	data: User | null;
	isGuest: boolean;
	updateUser: (updates: Partial<User>) => Promise<User | undefined>;
	updateUserSettings: (newSettings: User['settings']) => Promise<User['settings'] | undefined>;
	signOut: () => Promise<void>;
};

function useUser(): useUser {
	const { data, update } = useSession();
	const user = useMemo(() => data?.db, [data]);
	const isGuest = useMemo(() => !user?.role || user?.role?.length === 0, [user]);

	/**
	 * Update user
	 * Uses current auth provider's updateUser method
	 */
	async function handleUpdateUser(_data: Partial<User>) {
		const response = await authUpdateDbUser(_data);

		if (!response.ok) {
			throw new Error('Failed to update user');
		}

		const updatedUser = (await response.json()) as User;

		// Update AuthJs session data
		setTimeout(() => {
			update();
		}, 300);

		return updatedUser;
	}

	/**
	 * Update user settings
	 * Uses current auth provider's updateUser method
	 */
	async function handleUpdateUserSettings(newSettings: User['settings']) {
		const newUser = setIn(user, 'settings', newSettings) as User;

		if (_.isEqual(user, newUser)) {
			return undefined;
		}

		const updatedUser = await handleUpdateUser(newUser);

		return updatedUser?.settings;
	}

	/**
	 * Sign out
	 */
	async function handleSignOut() {
		return signOut();
	}

	return {
		data: user,
		isGuest,
		signOut: handleSignOut,
		updateUser: handleUpdateUser,
		updateUserSettings: handleUpdateUserSettings
	};
}

export default useUser;
