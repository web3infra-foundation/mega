'use client';

import authRoles from '@auth/authRoles';
import AuthGuardRedirect from '@auth/AuthGuardRedirect';
import SignUpPage from './SignUpPage';

function Page() {
	return (
		<AuthGuardRedirect auth={authRoles.onlyGuest}>
			<SignUpPage />
		</AuthGuardRedirect>
	);
}

export default Page;
