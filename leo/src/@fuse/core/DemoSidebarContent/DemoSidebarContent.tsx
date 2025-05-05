import _ from 'lodash';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemText from '@mui/material/ListItemText';
import { cloneElement, memo, type JSX } from 'react';

/**
 * This component renders a demo sidebar content which contains List with ListItemButton and ListItemText for -30- times.
 */
function DemoSidebarContent() {
	function generate(element: JSX.Element) {
		return _(30).times((value) =>
			cloneElement(element, {
				key: value
			})
		);
	}

	return (
		<div>
			<List dense>
				{generate(
					<ListItemButton>
						<ListItemText primary="Single-line item" />
					</ListItemButton>
				)}
			</List>
		</div>
	);
}

export default memo(DemoSidebarContent);
