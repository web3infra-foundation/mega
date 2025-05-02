import * as React from 'react';
import { isNodeSelection, type Editor } from '@tiptap/react';

// --- Hooks ---
import { useTiptapEditor } from '@/hooks/use-tiptap-editor';

// --- Icons ---
import { ChevronDownIcon } from '@/components/tiptap/tiptap-icons/chevron-down-icon';
import { ListIcon } from '@/components/tiptap/tiptap-icons/list-icon';

// --- Lib ---
import { isNodeInSchema } from '@/lib/tiptap-utils';

// --- Tiptap UI ---
import {
	ListButton,
	canToggleList,
	isListActive,
	listOptions,
	type ListType
} from '@/components/tiptap/tiptap-ui/list-button/list-button';

// --- UI Primitives ---
import { Button, ButtonProps } from '@/components/tiptap/tiptap-ui-primitive/button';
import {
	DropdownMenu,
	DropdownMenuTrigger,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem
} from '@/components/tiptap/tiptap-ui-primitive/dropdown-menu';

export interface ListDropdownMenuProps extends Omit<ButtonProps, 'type'> {
	/**
	 * The TipTap editor instance.
	 */
	editor?: Editor;
	/**
	 * The list types to display in the dropdown.
	 */
	types?: ListType[];
	/**
	 * Whether the dropdown should be hidden when no list types are available
	 * @default false
	 */
	hideWhenUnavailable?: boolean;
	onOpenChange?: (isOpen: boolean) => void;
}

export function canToggleAnyList(editor: Editor | null, listTypes: ListType[]): boolean {
	if (!editor) return false;

	return listTypes.some((type) => canToggleList(editor, type));
}

export function isAnyListActive(editor: Editor | null, listTypes: ListType[]): boolean {
	if (!editor) return false;

	return listTypes.some((type) => isListActive(editor, type));
}

export function getFilteredListOptions(availableTypes: ListType[]): typeof listOptions {
	return listOptions.filter((option) => !option.type || availableTypes.includes(option.type));
}

export function shouldShowListDropdown(params: {
	editor: Editor | null;
	listTypes: ListType[];
	hideWhenUnavailable: boolean;
	listInSchema: boolean;
	canToggleAny: boolean;
}): boolean {
	const { editor, hideWhenUnavailable, listInSchema, canToggleAny } = params;

	if (!listInSchema) {
		return false;
	}

	if (hideWhenUnavailable) {
		if (isNodeSelection(editor?.state.selection) && !canToggleAny) {
			return false;
		}
	}

	return true;
}

export function useListDropdownState(editor: Editor | null, availableTypes: ListType[]) {
	const [isOpen, setIsOpen] = React.useState(false);

	const listInSchema = availableTypes.some((type) => isNodeInSchema(type, editor));

	const filteredLists = React.useMemo(() => getFilteredListOptions(availableTypes), [availableTypes]);

	const canToggleAny = canToggleAnyList(editor, availableTypes);
	const isAnyActive = isAnyListActive(editor, availableTypes);

	const handleOpenChange = React.useCallback((open: boolean, callback?: (isOpen: boolean) => void) => {
		setIsOpen(open);
		callback?.(open);
	}, []);

	return {
		isOpen,
		setIsOpen,
		listInSchema,
		filteredLists,
		canToggleAny,
		isAnyActive,
		handleOpenChange
	};
}

export function useActiveListIcon(editor: Editor | null, filteredLists: typeof listOptions) {
	return React.useCallback(() => {
		const activeOption = filteredLists.find((option) => isListActive(editor, option.type));

		return activeOption ? (
			<activeOption.icon className="tiptap-button-icon" />
		) : (
			<ListIcon className="tiptap-button-icon" />
		);
	}, [editor, filteredLists]);
}

export function ListDropdownMenu({
	editor: providedEditor,
	types = ['bulletList', 'orderedList', 'taskList'],
	hideWhenUnavailable = false,
	onOpenChange,
	...props
}: ListDropdownMenuProps) {
	const editor = useTiptapEditor(providedEditor);

	const { isOpen, listInSchema, filteredLists, canToggleAny, isAnyActive, handleOpenChange } = useListDropdownState(
		editor,
		types
	);

	const getActiveIcon = useActiveListIcon(editor, filteredLists);

	const show = React.useMemo(() => {
		return shouldShowListDropdown({
			editor,
			listTypes: types,
			hideWhenUnavailable,
			listInSchema,
			canToggleAny
		});
	}, [editor, types, hideWhenUnavailable, listInSchema, canToggleAny]);

	const handleOnOpenChange = React.useCallback(
		(open: boolean) => handleOpenChange(open, onOpenChange),
		[handleOpenChange, onOpenChange]
	);

	if (!show || !editor || !editor.isEditable) {
		return null;
	}

	return (
		<DropdownMenu
			open={isOpen}
			onOpenChange={handleOnOpenChange}
		>
			<DropdownMenuTrigger asChild>
				<Button
					type="button"
					data-style="ghost"
					data-active-state={isAnyActive ? 'on' : 'off'}
					role="button"
					tabIndex={-1}
					aria-label="List options"
					tooltip="List"
					{...props}
				>
					{getActiveIcon()}
					<ChevronDownIcon className="tiptap-button-dropdown-small" />
				</Button>
			</DropdownMenuTrigger>

			<DropdownMenuContent>
				<DropdownMenuGroup>
					{filteredLists.map((option) => (
						<DropdownMenuItem
							key={option.type}
							asChild
						>
							<ListButton
								editor={editor}
								type={option.type}
								text={option.label}
								hideWhenUnavailable={hideWhenUnavailable}
								tooltip={''}
							/>
						</DropdownMenuItem>
					))}
				</DropdownMenuGroup>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

export default ListDropdownMenu;
