import * as React from 'react';
import { type Editor } from '@tiptap/react';

// --- Hooks ---
import { useTiptapEditor } from '@/hooks/use-tiptap-editor';

// --- Icons ---
import { ImagePlusIcon } from '@/components/tiptap/tiptap-icons/image-plus-icon';

// --- UI Primitives ---
import { Button, ButtonProps } from '@/components/tiptap/tiptap-ui-primitive/button';

export interface ImageUploadButtonProps extends ButtonProps {
	editor?: Editor | null;
	text?: string;
	extensionName?: string;
}

export function isImageActive(editor: Editor | null, extensionName: string): boolean {
	if (!editor) return false;

	return editor.isActive(extensionName);
}

export function insertImage(editor: Editor | null, extensionName: string): boolean {
	if (!editor) return false;

	return editor
		.chain()
		.focus()
		.insertContent({
			type: extensionName
		})
		.run();
}

export function useImageUploadButton(editor: Editor | null, extensionName = 'imageUpload', disabled = false) {
	const isActive = isImageActive(editor, extensionName);
	const handleInsertImage = React.useCallback(() => {
		if (disabled) return false;

		return insertImage(editor, extensionName);
	}, [editor, extensionName, disabled]);

	return {
		isActive,
		handleInsertImage
	};
}

export const ImageUploadButton = React.forwardRef<HTMLButtonElement, ImageUploadButtonProps>(
	(
		{
			editor: providedEditor,
			extensionName = 'imageUpload',
			text,
			className = '',
			disabled,
			onClick,
			children,
			...buttonProps
		},
		ref
	) => {
		const editor = useTiptapEditor(providedEditor);
		const { isActive, handleInsertImage } = useImageUploadButton(editor, extensionName, disabled);

		const handleClick = React.useCallback(
			(e: React.MouseEvent<HTMLButtonElement>) => {
				onClick?.(e);

				if (!e.defaultPrevented && !disabled) {
					handleInsertImage();
				}
			},
			[onClick, disabled, handleInsertImage]
		);

		if (!editor || !editor.isEditable) {
			return null;
		}

		return (
			<Button
				ref={ref}
				type="button"
				className={className.trim()}
				data-style="ghost"
				data-active-state={isActive ? 'on' : 'off'}
				role="button"
				tabIndex={-1}
				aria-label="Add image"
				aria-pressed={isActive}
				tooltip="Add image"
				onClick={handleClick}
				{...buttonProps}
			>
				{children || (
					<>
						<ImagePlusIcon className="tiptap-button-icon" />
						{text && <span className="tiptap-button-text">{text}</span>}
					</>
				)}
			</Button>
		);
	}
);

ImageUploadButton.displayName = 'ImageUploadButton';

export default ImageUploadButton;
