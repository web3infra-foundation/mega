import { usePathname, useRouter, useSearchParams } from 'next/navigation';
import { ComponentType, memo } from 'react';

export type WithRouterProps = {
	pathname?: ReturnType<typeof usePathname>;
	params?: Record<string, string>;
	router?: ReturnType<typeof useRouter>;
};

/**
 * The withRouter function is a higher-order component that wraps a component with the usePathname, useRouter, and useSearchParams hooks from Next.js.
 * It passes the pathname, params, and router objects as props to the wrapped component.
 * The component is memoized to prevent unnecessary re-renders.
 */
const withRouter = <Props extends WithRouterProps>(Component: ComponentType<Props>) =>
	memo(function WithRouterWrapper(props: Omit<Props, keyof WithRouterProps>) {
		const pathname = usePathname();
		const router = useRouter();
		const searchParams = useSearchParams();
		const params = Object.fromEntries(searchParams.entries());

		return (
			<Component
				{...(props as Props)}
				pathname={pathname}
				params={params}
				router={router}
			/>
		);
	});

export default withRouter;
