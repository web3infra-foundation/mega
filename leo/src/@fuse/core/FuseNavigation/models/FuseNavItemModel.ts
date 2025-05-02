import _ from 'lodash';
import { PartialDeep } from 'type-fest';
import { FuseNavItemType } from '../types/FuseNavItemType';

/**
 *  FuseNavItemModel
 *  Constructs a navigation item based on FuseNavItemType
 */
function FuseNavItemModel(data?: PartialDeep<FuseNavItemType>) {
	data = data || {};

	return _.defaults(data, {
		id: _.uniqueId(),
		title: '',
		translate: '',
		auth: null,
		subtitle: '',
		icon: '',
		iconClass: '',
		url: '',
		target: '',
		type: 'item',
		sx: {},
		disabled: false,
		active: false,
		exact: false,
		end: false,
		badge: null,
		children: []
	});
}

export default FuseNavItemModel;
