import clsx from 'clsx';
import { ReactNode } from 'react';

/**
 * Props for the FusePageSimpleHeader component.
 */
type FusePageSimpleHeaderProps = {
	className?: string;
	header?: ReactNode;
};

/**
 * The FusePageSimpleHeader component is a sub-component of the FusePageSimple layout component.
 * It provides a header area for the layout.
 */
function FusePageSimpleHeader(props: FusePageSimpleHeaderProps) {
	const { header = null, className } = props;
	return (
		<div className={clsx('FusePageSimple-header', className)}>
			<div className="container">{header}</div>
		</div>
	);
}

export default FusePageSimpleHeader;
