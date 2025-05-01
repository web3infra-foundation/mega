import { lazy, memo, Suspense } from 'react';

const QuickPanel = lazy(() => import('@/components/theme-layouts/components/quickPanel/QuickPanel'));

/**
 * The right side layout 2.
 */
function RightSideLayout2() {
	return (
		<Suspense>
			<QuickPanel />
		</Suspense>
	);
}

export default memo(RightSideLayout2);
