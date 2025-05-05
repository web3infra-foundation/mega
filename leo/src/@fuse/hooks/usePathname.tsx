'use client';

import { usePathname as usePath } from 'next/navigation';

function usePathname() {
	return usePath();
}

export default usePathname;
