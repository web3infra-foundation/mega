import { useEffect, useState } from 'react';

type FuseAwaitRenderProps = {
	delay?: number;
	children: React.ReactNode;
};

function FuseAwaitRender(props: FuseAwaitRenderProps) {
	const { delay = 0, children } = props;
	const [awaitRender, setAwaitRender] = useState(true);

	useEffect(() => {
		setTimeout(() => {
			setAwaitRender(false);
		}, delay);
	}, [delay]);

	return awaitRender ? null : children;
}

export default FuseAwaitRender;
