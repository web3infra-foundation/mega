import clsx from 'clsx';
import { ReactNode } from 'react';

/**
 * Props for the FusePageCardedHeader component.
 */
type FusePageCardedHeaderProps = {
	header?: ReactNode;
};

/**
 * The FusePageCardedHeader component is a header for the FusePageCarded component.
 */
function FusePageCardedHeader(props: FusePageCardedHeaderProps) {
	const { header = null } = props;

	return <div className={clsx('FusePageCarded-header', 'container')}>{header}</div>;
}

export default FusePageCardedHeader;
