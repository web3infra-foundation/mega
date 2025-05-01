import { Control } from 'react-hook-form';
import { debounce } from 'lodash';
import { Controller } from 'react-hook-form';
import { FuseSettingsConfigType } from '../FuseSettings';
import { useEffect, useState } from 'react';
import { useRef } from 'react';
import { TextField } from '@mui/material';

type NumberFormControllerProps = {
	name: keyof FuseSettingsConfigType;
	control: Control<FuseSettingsConfigType>;
	item: {
		title: string;
		min?: number;
		max?: number;
	};
};

function NumberFormController(props: NumberFormControllerProps) {
	const { name, control, item } = props;

	return (
		<div
			key={name}
			className="FuseSettings-formControl"
		>
			<Controller
				name={name}
				control={control}
				render={({ field: { onChange, value } }) => (
					<NumberTextField
						value={+value}
						onChange={onChange}
						item={item}
					/>
				)}
			/>
		</div>
	);
}

type NumberTextFieldProps = {
	value: number;
	onChange: (value: number) => void;
	item: NumberFormControllerProps['item'];
};

function NumberTextField(props: NumberTextFieldProps) {
	const { value, onChange, item } = props;
	const [localValue, setLocalValue] = useState(value);
	const [error, setError] = useState('');

	const debouncedOnChange = useRef(
		debounce((newValue: number) => {
			onChange(newValue);
		}, 500)
	).current;

	const handleChange = (ev: React.ChangeEvent<HTMLInputElement>) => {
		const newValue = +ev.target.value;

		if (item?.min && newValue < item?.min) {
			setError(`Value is too low (min: ${item?.min})`);
			setLocalValue(newValue);
			return;
		}

		if (item?.max && newValue > item?.max) {
			setError(`Value is too high (max: ${item?.max})`);
			setLocalValue(newValue);
			return;
		}

		setError('');
		setLocalValue(newValue);
		debouncedOnChange(newValue);
	};

	useEffect(() => {
		setLocalValue(value);
	}, [value]);

	return (
		<TextField
			value={localValue}
			onChange={handleChange}
			label={item.title}
			type="number"
			slotProps={{
				inputLabel: { shrink: true },
				input: {
					inputProps: { min: item.min, max: item.max }
				}
			}}
			variant="outlined"
			error={!!error}
			helperText={error}
		/>
	);
}

export default NumberFormController;
