import FuseScrollbars from '@fuse/core/FuseScrollbars';
import { ReactNode } from 'react';

/**
 * Props for the FusePageCardedSidebarContent component.
 */
type FusePageCardedSidebarContentProps = {
	innerScroll?: boolean;
	children?: ReactNode;
};

/**
 * The FusePageCardedSidebarContent component is a content container for the FusePageCardedSidebar component.
 */
function FusePageCardedSidebarContent(props: FusePageCardedSidebarContentProps) {
	const { innerScroll, children } = props;

	if (!children) {
		return null;
	}

	return (
		<FuseScrollbars enable={innerScroll}>
			<div className="FusePageCarded-sidebarContent min-w-80 lg:min-w-0">{children}</div>
		</FuseScrollbars>
	);
}

export default FusePageCardedSidebarContent;
