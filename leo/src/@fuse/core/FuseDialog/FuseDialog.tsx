import Dialog from '@mui/material/Dialog';
import { useAppDispatch, useAppSelector } from 'src/store/hooks';
import { closeDialog, selectFuseDialogProps } from '@fuse/core/FuseDialog/fuseDialogSlice';

/**
 * FuseDialog component
 * This component renders a material UI ```Dialog``` component
 * with properties pulled from the redux store
 */
function FuseDialog() {
	const dispatch = useAppDispatch();
	const options = useAppSelector(selectFuseDialogProps);

	return (
		<Dialog
			onClose={() => dispatch(closeDialog())}
			aria-labelledby="fuse-dialog-title"
			classes={{
				paper: 'rounded-lg'
			}}
			{...options}
		/>
	);
}

export default FuseDialog;
