import MainLayout from '../../components/MainLayout';

function Layout(props: { children: React.ReactNode }) {
	const { children } = props;

	return (
		<MainLayout
			navbar={false}
			toolbar={false}
			leftSidePanel={false}
			rightSidePanel={false}
			footer={false}
		>
			{children}
		</MainLayout>
	);
}

export default Layout;
