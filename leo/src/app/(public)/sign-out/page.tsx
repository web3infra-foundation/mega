'use client';

import AuthGuardRedirect from '@auth/AuthGuardRedirect';
import SignOutPage from './SignOutPage';

function Page() {
	return (
		<AuthGuardRedirect auth={null}>
			<SignOutPage />
		</AuthGuardRedirect>
	);
}

export default Page;
