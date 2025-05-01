import * as React from 'react';
import { isNodeSelection, type Editor } from '@tiptap/react';

// --- Hooks ---
import { useTiptapEditor } from '@/hooks/use-tiptap-editor';

// --- Icons ---
import { BlockQuoteIcon } from '@/components/tiptap/tiptap-icons/block-quote-icon';
import { CodeBlockIcon } from '@/components/tiptap/tiptap-icons/code-block-icon';

// --- Lib ---
import { isNodeInSchema } from '@/lib/tiptap-utils';

// --- UI Primitives ---
import { Button, ButtonProps } from '@/components/tiptap/tiptap-ui-primitive/button';

export type NodeType = 'codeBlock' | 'blockquote';

export interface NodeButtonProps extends Omit<ButtonProps, 'type'> {
	/**
	 * The TipTap editor instance.
	 */
	editor?: Editor | null;
	/**
	 * The type of node to toggle.
	 */
	type: NodeType;
	/**
	 * Optional text to display alongside the icon.
	 */
	text?: string;
	/**
	 * Whether the button should hide when the node is not available.
	 * @default false
	 */
	hideWhenUnavailable?: boolean;
}

export const nodeIcons = {
	codeBlock: CodeBlockIcon,
	blockquote: BlockQuoteIcon
};

export const nodeShortcutKeys: Partial<Record<NodeType, string>> = {
	codeBlock: 'Ctrl-Alt-c',
	blockquote: 'Ctrl-Shift-b'
};

export const nodeLabels: Record<NodeType, string> = {
	codeBlock: 'Code Block',
	blockquote: 'Blockquote'
};

export function canToggleNode(editor: Editor | null, type: NodeType): boolean {
	if (!editor) return false;

	try {
		return type === 'codeBlock'
			? editor.can().toggleNode('codeBlock', 'paragraph')
			: editor.can().toggleWrap('blockquote');
	} catch {
		return false;
	}
}

export function isNodeActive(editor: Editor | null, type: NodeType): boolean {
	if (!editor) return false;

	return editor.isActive(type);
}

export function toggleNode(editor: Editor | null, type: NodeType): boolean {
	if (!editor) return false;

	if (type === 'codeBlock') {
		return editor.chain().focus().toggleNode('codeBlock', 'paragraph').run();
	} else {
		return editor.chain().focus().toggleWrap('blockquote').run();
	}
}

export function isNodeButtonDisabled(editor: Editor | null, canToggle: boolean, userDisabled = false): boolean {
	if (!editor) return true;

	if (userDisabled) return true;

	if (!canToggle) return true;

	return false;
}

export function shouldShowNodeButton(params: {
	editor: Editor | null;
	type: NodeType;
	hideWhenUnavailable: boolean;
	nodeInSchema: boolean;
	canToggle: boolean;
}): boolean {
	const { editor, hideWhenUnavailable, nodeInSchema, canToggle } = params;

	if (!nodeInSchema) {
		return false;
	}

	if (hideWhenUnavailable) {
		if (isNodeSelection(editor?.state.selection) || !canToggle) {
			return false;
		}
	}

	return Boolean(editor?.isEditable);
}

export function formatNodeName(type: NodeType): string {
	return type.charAt(0).toUpperCase() + type.slice(1);
}

export function useNodeState(editor: Editor | null, type: NodeType, disabled = false, hideWhenUnavailable = false) {
	const nodeInSchema = isNodeInSchema(type, editor);

	const canToggle = canToggleNode(editor, type);
	const isDisabled = isNodeButtonDisabled(editor, canToggle, disabled);
	const isActive = isNodeActive(editor, type);

	const shouldShow = React.useMemo(
		() =>
			shouldShowNodeButton({
				editor,
				type,
				hideWhenUnavailable,
				nodeInSchema,
				canToggle
			}),
		[editor, type, hideWhenUnavailable, nodeInSchema, canToggle]
	);

	const handleToggle = React.useCallback(() => {
		if (!isDisabled && editor) {
			return toggleNode(editor, type);
		}

		return false;
	}, [editor, type, isDisabled]);

	const Icon = nodeIcons[type];
	const shortcutKey = nodeShortcutKeys[type];
	const label = nodeLabels[type];

	return {
		nodeInSchema,
		canToggle,
		isDisabled,
		isActive,
		shouldShow,
		handleToggle,
		Icon,
		shortcutKey,
		label
	};
}

export const NodeButton = React.forwardRef<HTMLButtonElement, NodeButtonProps>(
	(
		{
			editor: providedEditor,
			type,
			text,
			hideWhenUnavailable = false,
			className = '',
			disabled,
			onClick,
			children,
			...buttonProps
		},
		ref
	) => {
		const editor = useTiptapEditor(providedEditor);

		const { isDisabled, isActive, shouldShow, handleToggle, Icon, shortcutKey, label } = useNodeState(
			editor,
			type,
			disabled,
			hideWhenUnavailable
		);

		const handleClick = React.useCallback(
			(e: React.MouseEvent<HTMLButtonElement>) => {
				onClick?.(e);

				if (!e.defaultPrevented && !isDisabled) {
					handleToggle();
				}
			},
			[onClick, isDisabled, handleToggle]
		);

		if (!shouldShow || !editor || !editor.isEditable) {
			return null;
		}

		return (
			<Button
				type="button"
				className={className.trim()}
				disabled={isDisabled}
				data-style="ghost"
				data-active-state={isActive ? 'on' : 'off'}
				data-disabled={isDisabled}
				role="button"
				tabIndex={-1}
				aria-label={type}
				aria-pressed={isActive}
				tooltip={label}
				shortcutKeys={shortcutKey}
				onClick={handleClick}
				{...buttonProps}
				ref={ref}
			>
				{children || (
					<>
						<Icon className="tiptap-button-icon" />
						{text && <span className="tiptap-button-text">{text}</span>}
					</>
				)}
			</Button>
		);
	}
);

NodeButton.displayName = 'NodeButton';

export default NodeButton;
