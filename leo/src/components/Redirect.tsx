import { redirect } from 'next/navigation';

type RedirectProps = {
	to: string;
	children?: React.ReactNode;
};

function Redirect(props: RedirectProps) {
	const { to, children = null } = props;

	redirect(to);

	return children;
}

export default Redirect;
