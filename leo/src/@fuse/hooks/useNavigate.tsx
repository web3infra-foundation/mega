'use client';

import { useRouter } from 'next/navigation';

function useNavigate(): (url: string | number) => void {
	const router = useRouter();

	return (url) => {
		if (typeof url === 'string') {
			router.push(url);
		}

		if (url === -1) {
			router.back();
		}

		if (url === 1) {
			router.forward();
		}
	};
}

export default useNavigate;
