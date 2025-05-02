import { MouseEvent, ReactNode, useEffect, useState } from 'react';
import { Controller, useForm } from 'react-hook-form';
import _ from 'lodash';
import { darkPaletteText, lightPaletteText } from 'src/configs/themesConfig';
import { Theme, darken, getContrastRatio, lighten } from '@mui/material/styles';
import { useTheme } from '@mui/system';
import AppBar from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import { Dialog, DialogActions, DialogContent, Icon, TextField } from '@mui/material';
import Typography from '@mui/material/Typography';
import ButtonGroup from '@mui/material/ButtonGroup';
import Button from '@mui/material/Button';
import PalettePreview from './PalettePreview';
import SectionPreview from './SectionPreview';
import { FuseThemeType } from '../FuseSettings';

/**
 * Check if a color is dark
 */
function isDark(color: string) {
	return getContrastRatio(color, '#ffffff') >= 3;
}

/**
 * Props for PaletteSelector component
 */
type PaletteSelectorProps = {
	triggerElement: ReactNode;
	value: FuseThemeType;
	onChange: (value: FuseThemeType) => void;
};

/**
 * PaletteSelector component
 */
function PaletteSelector(props: PaletteSelectorProps) {
	const {
		value,
		onChange,
		triggerElement = (
			<div className="m-2 flex w-32 flex-col items-center space-y-2">
				<SectionPreview />
				<Typography className="mb-6 flex-1 text-lg font-bold">Edit Palette</Typography>
			</div>
		)
	} = props;
	const [openDialog, setOpenDialog] = useState(false);

	const theme: Theme = useTheme();

	const methods = useForm<FuseThemeType>({
		defaultValues: {},
		mode: 'onChange'
	});

	const { reset, formState, trigger, handleSubmit, watch, control, setValue } = methods;

	const { isValid, dirtyFields, errors } = formState;

	useEffect(() => {
		reset(value);
	}, [value, reset]);

	const form = watch();

	const formType = watch('palette.mode');

	useEffect(() => {
		if (!formType || !openDialog) {
			return;
		}

		setTimeout(() => {
			trigger(['palette.background.paper', 'palette.background.default']);
		});
	}, [formType, openDialog, trigger]);

	const backgroundColorValidation = (colorVal: string) => {
		if (formType === 'light' && isDark(colorVal)) {
			return 'Must be a light color';
		}

		if (formType === 'dark' && !isDark(colorVal)) {
			return 'Must be a dark color';
		}

		return true;
	};

	/**
	 * Open Dialog
	 */
	function handleOpenDialog(ev: MouseEvent<HTMLDivElement>) {
		ev.preventDefault();
		ev.stopPropagation();
		setOpenDialog(true);
	}

	/**
	 * Close Dialog
	 */
	function handleCloseDialog() {
		setOpenDialog(false);
	}

	function onSubmit(formData: FuseThemeType) {
		onChange(formData);
		handleCloseDialog();
	}

	return (
		<>
			<div
				onClick={handleOpenDialog}
				role="button"
			>
				{triggerElement}
			</div>
			<Dialog
				container={document.body}
				open={openDialog}
				onClose={handleCloseDialog}
				aria-labelledby="form-dialog-title"
				classes={{
					paper: 'rounded-5 w-full'
				}}
			>
				<AppBar
					elevation={0}
					position="static"
				>
					<Toolbar className="flex w-full">
						<Icon className="mr-3">palette</Icon>
						<Typography
							variant="subtitle1"
							color="inherit"
						>
							Edit palette
						</Typography>
					</Toolbar>
				</AppBar>

				<DialogContent>
					<div className="flex w-full">
						<div className="flex flex-1 flex-col items-center justify-center p-6">
							<Controller
								name="palette.mode"
								control={control}
								render={({ field: { onChange: _onChange, value: _value } }) => (
									<ButtonGroup
										disableElevation
										variant="contained"
										color="secondary"
										className="mb-8"
									>
										<Button
											onClick={() => {
												_onChange('light');
												setValue('palette.text', lightPaletteText, { shouldDirty: true });
											}}
											variant={_value === 'light' ? 'contained' : 'outlined'}
										>
											Light
										</Button>

										<Button
											onClick={() => {
												_onChange('dark');
												setValue('palette.text', darkPaletteText, { shouldDirty: true });
											}}
											variant={_value === 'dark' ? 'contained' : 'outlined'}
										>
											Dark
										</Button>
									</ButtonGroup>
								)}
							/>

							<Controller
								name="palette.primary.main"
								control={control}
								render={({ field: { onChange: _onChange, value: _value } }) => (
									<TextField
										value={_value}
										onChange={(ev) => {
											_onChange(ev.target.value);
											setValue('palette.primary.light', lighten(ev.target.value, 0.8), {
												shouldDirty: true
											});
											setValue('palette.primary.dark', darken(ev.target.value, 0.2), {
												shouldDirty: true
											});
											setValue(
												'palette.primary.contrastText',
												theme.palette.getContrastText(ev.target.value),
												{ shouldDirty: true }
											);
										}}
										type="color"
										variant="outlined"
										className="mb-8"
										label="Primary color"
										slotProps={{
											input: { className: 'w-50  h-8' }
										}}
									/>
								)}
							/>

							<Controller
								name="palette.secondary.main"
								control={control}
								render={({ field: { onChange: _onChange, value: _value } }) => (
									<TextField
										value={_value}
										onChange={(ev) => {
											_onChange(ev.target.value);
											setValue('palette.secondary.light', lighten(ev.target.value, 0.8), {
												shouldDirty: true
											});
											setValue('palette.secondary.dark', darken(ev.target.value, 0.2), {
												shouldDirty: true
											});
											setValue(
												'palette.secondary.contrastText',
												theme.palette.getContrastText(ev.target.value),
												{ shouldDirty: true }
											);
										}}
										type="color"
										variant="outlined"
										className="mb-8"
										label="Secondary color"
										slotProps={{
											input: { className: 'w-50 h-8' }
										}}
									/>
								)}
							/>

							<Controller
								name="palette.background.paper"
								control={control}
								rules={{
									validate: {
										backgroundColorValidation
									}
								}}
								render={({ field }) => (
									<TextField
										{...field}
										type="color"
										variant="outlined"
										className="mb-8"
										label="Background paper"
										slotProps={{
											input: { className: 'w-50 h-8' }
										}}
										error={!!errors?.palette?.background?.paper}
										helperText={errors?.palette?.background?.paper?.message}
									/>
								)}
							/>

							<Controller
								name="palette.background.default"
								control={control}
								rules={{
									validate: {
										backgroundColorValidation
									}
								}}
								render={({ field }) => (
									<TextField
										{...field}
										type="color"
										variant="outlined"
										label="Background default"
										slotProps={{
											input: { className: 'w-50 h-8' }
										}}
										error={!!errors?.palette?.background?.default}
										helperText={errors?.palette?.background?.default?.message}
									/>
								)}
							/>
						</div>

						<div className="flex flex-col items-center justify-center p-12">
							<Typography
								className="-mt-12 mb-4 text-lg font-semibold"
								color="text.secondary"
							>
								Preview
							</Typography>
							<PalettePreview palette={form.palette} />
						</div>
					</div>
				</DialogContent>
				<DialogActions className="flex justify-between p-4">
					<Button
						onClick={handleCloseDialog}
						color="primary"
					>
						Cancel
					</Button>
					<Button
						variant="contained"
						color="secondary"
						autoFocus
						onClick={handleSubmit(onSubmit)}
						disabled={_.isEmpty(dirtyFields) || !isValid}
					>
						Save
					</Button>
				</DialogActions>
			</Dialog>
		</>
	);
}

export default PaletteSelector;
