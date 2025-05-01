import Tooltip from '@mui/material/Tooltip';
import clsx from 'clsx';
import { useEffect, useLayoutEffect, useState } from 'react';
import IconButton from '@mui/material/IconButton';
import FuseSvgIcon from '@fuse/core/FuseSvgIcon';

const useEnhancedEffect = typeof window !== 'undefined' ? useLayoutEffect : useEffect;

type FullScreenDocumentType = Document & {
	mozCancelFullScreen?: () => void;
	msExitFullscreen?: () => void;
	webkitExitFullscreen?: () => void;
	mozFullScreenElement?: Element | null;
	msFullscreenElement?: Element | null;
	webkitFullscreenElement?: Element | null;
};

type FullScreenHTMLElementType = HTMLElement & {
	mozRequestFullScreen?: () => void;
	webkitRequestFullscreen?: () => void;
	msRequestFullscreen?: () => void;
};

type HeaderFullScreenToggleProps = {
	className?: string;
};

/**
 * The header full screen toggle.
 */
function HeaderFullScreenToggle(props: HeaderFullScreenToggleProps) {
	const { className = '' } = props;

	const [isFullScreen, setIsFullScreen] = useState(false);

	useEnhancedEffect(() => {
		document.onfullscreenchange = () =>
			setIsFullScreen((document as FullScreenDocumentType)[getBrowserFullscreenElementProp()] != null);

		return () => {
			document.onfullscreenchange = null;
		};
	});

	function getBrowserFullscreenElementProp(): keyof FullScreenDocumentType {
		const doc: FullScreenDocumentType = document as FullScreenDocumentType;

		if (typeof doc.fullscreenElement !== 'undefined') {
			return 'fullscreenElement';
		}

		if (typeof doc.mozFullScreenElement !== 'undefined') {
			return 'mozFullScreenElement';
		}

		if (typeof doc.msFullscreenElement !== 'undefined') {
			return 'msFullscreenElement';
		}

		if (typeof doc.webkitFullscreenElement !== 'undefined') {
			return 'webkitFullscreenElement';
		}

		throw new Error('fullscreenElement is not supported by this browser');
	}

	/* View in fullscreen */
	function openFullscreen() {
		const elem: FullScreenHTMLElementType = document.documentElement;

		if (elem.requestFullscreen) {
			elem.requestFullscreen();
		} else if (elem.mozRequestFullScreen) {
			/* Firefox */
			elem.mozRequestFullScreen();
		} else if (elem.webkitRequestFullscreen) {
			/* Chrome, Safari and Opera */
			elem.webkitRequestFullscreen();
		} else if (elem.msRequestFullscreen) {
			/* IE/Edge */
			elem.msRequestFullscreen();
		}
	}

	/* Close fullscreen */
	function closeFullscreen() {
		const doc: FullScreenDocumentType = document;

		if (doc.exitFullscreen) {
			doc.exitFullscreen();
		} else if (doc.mozCancelFullScreen) {
			/* Firefox */
			doc.mozCancelFullScreen();
		} else if (doc.webkitExitFullscreen) {
			/* Chrome, Safari and Opera */
			doc.webkitExitFullscreen();
		} else if (doc.msExitFullscreen) {
			/* IE/Edge */
			doc.msExitFullscreen();
		}
	}

	function toggleFullScreen() {
		const doc: FullScreenDocumentType = document;
		const elem: FullScreenHTMLElementType = document.documentElement;

		if (doc.fullscreenElement || doc.webkitFullscreenElement || doc.mozFullScreenElement) {
			closeFullscreen();
		} else if (
			elem.requestFullscreen ||
			elem.mozRequestFullScreen ||
			elem.webkitRequestFullscreen ||
			elem.msRequestFullscreen
		) {
			openFullscreen();
		}
	}

	return (
		<Tooltip
			title="Fullscreen toggle"
			placement="bottom"
		>
			<IconButton
				onClick={toggleFullScreen}
				className={clsx('border border-divider', className, isFullScreen && 'text-red-500')}
			>
				<FuseSvgIcon size={20}>heroicons-outline:arrows-pointing-out</FuseSvgIcon>
			</IconButton>
		</Tooltip>
	);
}

export default HeaderFullScreenToggle;
