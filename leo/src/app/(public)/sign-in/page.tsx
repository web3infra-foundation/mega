'use client';

import authRoles from '@auth/authRoles';
import AuthGuardRedirect from '@auth/AuthGuardRedirect';
import SignInPage from './SignInPage';

function Page() {
	return (
		<AuthGuardRedirect auth={authRoles.onlyGuest}>
			<SignInPage />
		</AuthGuardRedirect>
	);
}

export default Page;
