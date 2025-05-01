import { redirect } from 'next/navigation';

function MainPage() {
	redirect(`/dashboard`);
	return null;
}

export default MainPage;
