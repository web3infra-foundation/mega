import { Control } from 'react-hook-form';
import FuseLayoutConfig from './FuseLayoutConfig';
import ThemeFormConfigTypes from './ThemeFormConfigTypes';
import { FuseSettingsConfigType } from './FuseSettings';

type FuseSettingsControllersProps = {
	value: ThemeFormConfigTypes;
	prefix: string;
	control: Control<FuseSettingsConfigType>;
};

function FuseLayoutConfigs(props: FuseSettingsControllersProps) {
	const { value, prefix, control } = props;

	return Object?.entries?.(value)?.map?.(([key, item]) => {
		const name = prefix ? `${prefix}.${key}` : key;
		return (
			<FuseLayoutConfig
				key={key}
				name={name as keyof FuseSettingsConfigType}
				control={control}
				item={item}
			/>
		);
	});
}

export default FuseLayoutConfigs;
