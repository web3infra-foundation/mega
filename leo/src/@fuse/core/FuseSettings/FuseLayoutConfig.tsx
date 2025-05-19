import { Control } from 'react-hook-form';
import { Typography } from '@mui/material';
import { AnyFormFieldType } from '@fuse/core/FuseSettings/ThemeFormConfigTypes';
import { FuseSettingsConfigType } from '@fuse/core/FuseSettings/FuseSettings';
import FuseLayoutConfigs from './FuseLayoutConfigs';
import RadioFormController from './form-controllers/RadioFormController';
import SwitchFormController from './form-controllers/SwitchFormController';
import NumberFormController from './form-controllers/NumberFormController';

type FuseSettingsControllerProps = {
	key?: string;
	name: keyof FuseSettingsConfigType;
	control: Control<FuseSettingsConfigType>;
	title?: string;
	item: AnyFormFieldType;
};

function FuseLayoutConfig(props: FuseSettingsControllerProps) {
	const { item, name, control } = props;

	switch (item.type) {
		case 'radio':
			return (
				<RadioFormController
					name={name}
					control={control}
					item={item}
				/>
			);
		case 'switch':
			return (
				<SwitchFormController
					name={name}
					control={control}
					item={item}
				/>
			);
		case 'number':
			return (
				<NumberFormController
					name={name}
					control={control}
					item={item}
				/>
			);
		case 'group':
			return (
				<div
					key={name}
					className="FuseSettings-formGroup"
				>
					<Typography
						className="FuseSettings-formGroupTitle"
						color="text.secondary"
					>
						{item.title}
					</Typography>
					<FuseLayoutConfigs
						value={item.children}
						prefix={name}
						control={control}
					/>
				</div>
			);
		default:
			return '';
	}
}

export default FuseLayoutConfig;
