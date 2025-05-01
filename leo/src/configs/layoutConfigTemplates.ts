import { DeepPartial } from 'react-hook-form';
import { FuseSettingsConfigType } from '@fuse/core/FuseSettings/FuseSettings';

export const layoutConfigOnlyMain: DeepPartial<FuseSettingsConfigType>['layout'] = {
	config: {
		navbar: {
			display: false
		},
		toolbar: {
			display: false
		},
		footer: {
			display: false
		},
		leftSidePanel: {
			display: false
		},
		rightSidePanel: {
			display: false
		}
	}
};

export const layoutConfigOnlyMainFullWidth: DeepPartial<FuseSettingsConfigType>['layout'] = {
	config: {
		...layoutConfigOnlyMain.config,
		mode: 'fullwidth'
	}
};

export const layoutNoContainer: DeepPartial<FuseSettingsConfigType>['layout'] = {
	config: {
		mode: 'fullwidth'
	}
};
