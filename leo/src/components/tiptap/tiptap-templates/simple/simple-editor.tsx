import * as React from 'react';
import { EditorContent, EditorContext, useEditor } from '@tiptap/react';

// --- Tiptap Core Extensions ---
import { StarterKit } from '@tiptap/starter-kit';
import { Image } from '@tiptap/extension-image';
import { TaskItem } from '@tiptap/extension-task-item';
import { TaskList } from '@tiptap/extension-task-list';
import { TextAlign } from '@tiptap/extension-text-align';
import { Typography } from '@tiptap/extension-typography';
import { Highlight } from '@tiptap/extension-highlight';
import { Subscript } from '@tiptap/extension-subscript';
import { Superscript } from '@tiptap/extension-superscript';
import { Underline } from '@tiptap/extension-underline';

// --- Custom Extensions ---
import { Link } from '@/components/tiptap/tiptap-extension/link-extension';
import { Selection } from '@/components/tiptap/tiptap-extension/selection-extension';
import { TrailingNode } from '@/components/tiptap/tiptap-extension/trailing-node-extension';

// --- UI Primitives ---
import { Button } from '@/components/tiptap/tiptap-ui-primitive/button';
import { Spacer } from '@/components/tiptap/tiptap-ui-primitive/spacer';
import { Toolbar, ToolbarGroup, ToolbarSeparator } from '@/components/tiptap/tiptap-ui-primitive/toolbar';

// --- Tiptap Node ---
import { ImageUploadNode } from '@/components/tiptap/tiptap-node/image-upload-node/image-upload-node-extension';
import '@/components/tiptap/tiptap-node/code-block-node/code-block-node.scss';
import '@/components/tiptap/tiptap-node/list-node/list-node.scss';
import '@/components/tiptap/tiptap-node/image-node/image-node.scss';
import '@/components/tiptap/tiptap-node/paragraph-node/paragraph-node.scss';

// --- Tiptap UI ---
import { HeadingDropdownMenu } from '@/components/tiptap/tiptap-ui/heading-dropdown-menu';
import { ImageUploadButton } from '@/components/tiptap/tiptap-ui/image-upload-button';
import { ListDropdownMenu } from '@/components/tiptap/tiptap-ui/list-dropdown-menu';
import { NodeButton } from '@/components/tiptap/tiptap-ui/node-button';
import { HighlightPopover, HighlightContent, HighlighterButton } from '@/components/tiptap/tiptap-ui/highlight-popover';
import { LinkPopover, LinkContent, LinkButton } from '@/components/tiptap/tiptap-ui/link-popover';
import { MarkButton } from '@/components/tiptap/tiptap-ui/mark-button';
import { TextAlignButton } from '@/components/tiptap/tiptap-ui/text-align-button';
import { UndoRedoButton } from '@/components/tiptap/tiptap-ui/undo-redo-button';

// --- Icons ---
import { ArrowLeftIcon } from '@/components/tiptap/tiptap-icons/arrow-left-icon';
import { HighlighterIcon } from '@/components/tiptap/tiptap-icons/highlighter-icon';
import { LinkIcon } from '@/components/tiptap/tiptap-icons/link-icon';

// --- Hooks ---
import { useMobile } from '@/hooks/use-mobile';
import { useWindowSize } from '@/hooks/use-window-size';

// --- Lib ---
import { handleImageUpload, MAX_FILE_SIZE } from '@/lib/tiptap-utils';

// --- Styles ---
import '@/components/tiptap/tiptap-templates/simple/simple-editor.scss';

interface SimpleEditorProps {
	className?: string;
	value?: string;
	onChange?: (value: string) => void;
	error?: string;
	required?: boolean;
}

const MainToolbarContent = ({
	onHighlighterClick,
	onLinkClick,
	isMobile
}: {
	onHighlighterClick: () => void;
	onLinkClick: () => void;
	isMobile: boolean;
}) => {
	return (
		<>
			<Spacer />

			<ToolbarGroup>
				<UndoRedoButton action="undo" />
				<UndoRedoButton action="redo" />
			</ToolbarGroup>

			<ToolbarSeparator />

			<ToolbarGroup>
				<HeadingDropdownMenu levels={[1, 2, 3, 4]} />
				<ListDropdownMenu types={['bulletList', 'orderedList', 'taskList']} />
				<NodeButton type="codeBlock" />
				<NodeButton type="blockquote" />
			</ToolbarGroup>

			<ToolbarSeparator />

			<ToolbarGroup>
				<MarkButton type="bold" />
				<MarkButton type="italic" />
				<MarkButton type="strike" />
				<MarkButton type="code" />
				<MarkButton type="underline" />
				{!isMobile ? <HighlightPopover /> : <HighlighterButton onClick={onHighlighterClick} />}
				{!isMobile ? <LinkPopover /> : <LinkButton onClick={onLinkClick} />}
			</ToolbarGroup>

			<ToolbarSeparator />

			<ToolbarGroup>
				<MarkButton type="superscript" />
				<MarkButton type="subscript" />
			</ToolbarGroup>

			<ToolbarSeparator />

			<ToolbarGroup>
				<TextAlignButton align="left" />
				<TextAlignButton align="center" />
				<TextAlignButton align="right" />
				<TextAlignButton align="justify" />
			</ToolbarGroup>

			<ToolbarSeparator />

			<ToolbarGroup>
				<ImageUploadButton text="Add" />
			</ToolbarGroup>

			<Spacer />

			{isMobile && <ToolbarSeparator />}
		</>
	);
};

const MobileToolbarContent = ({ type, onBack }: { type: 'highlighter' | 'link'; onBack: () => void }) => (
	<>
		<ToolbarGroup>
			<Button
				data-style="ghost"
				onClick={onBack}
			>
				<ArrowLeftIcon className="tiptap-button-icon" />
				{type === 'highlighter' ? (
					<HighlighterIcon className="tiptap-button-icon" />
				) : (
					<LinkIcon className="tiptap-button-icon" />
				)}
			</Button>
		</ToolbarGroup>

		<ToolbarSeparator />

		{type === 'highlighter' ? <HighlightContent /> : <LinkContent />}
	</>
);

export function SimpleEditor({ value, onChange, error, className }: SimpleEditorProps) {
	const isMobile = useMobile();
	const windowSize = useWindowSize();
	const [mobileView, setMobileView] = React.useState<'main' | 'highlighter' | 'link'>('main');
	const [rect, setRect] = React.useState<Pick<DOMRect, 'x' | 'y' | 'width' | 'height'>>({
		x: 0,
		y: 0,
		width: 0,
		height: 0
	});
	const toolbarRef = React.useRef<HTMLDivElement>(null);

	React.useEffect(() => {
		const updateRect = () => {
			setRect(document.body.getBoundingClientRect());
		};

		updateRect();

		const resizeObserver = new ResizeObserver(updateRect);
		resizeObserver.observe(document.body);

		window.addEventListener('scroll', updateRect);

		return () => {
			resizeObserver.disconnect();
			window.removeEventListener('scroll', updateRect);
		};
	}, []);

	const editor = useEditor({
		immediatelyRender: false,
		content: value || '',
		onUpdate: ({ editor }) => {
			const html = editor.getHTML();
			onChange?.(html);
		},
		editorProps: {
			attributes: {
				autocomplete: 'off',
				autocorrect: 'off',
				autocapitalize: 'off',
				'aria-label': 'Main content area, start typing to enter text.'
			}
		},
		extensions: [
			StarterKit,
			TextAlign.configure({ types: ['heading', 'paragraph'] }),
			Underline,
			TaskList,
			TaskItem.configure({ nested: true }),
			Highlight.configure({ multicolor: true }),
			Image,
			Typography,
			Superscript,
			Subscript,

			Selection,
			ImageUploadNode.configure({
				accept: 'image/*',
				maxSize: MAX_FILE_SIZE,
				limit: 3,
				upload: handleImageUpload,
				onError: (error) => console.error('Upload failed:', error)
			}),
			TrailingNode,
			Link.configure({ openOnClick: false })
		]
	});

	React.useEffect(() => {
		if (editor && value !== undefined && value !== editor.getHTML()) {
			editor.commands.setContent(value);
		}
	}, [editor, value]);

	React.useEffect(() => {
		const checkCursorVisibility = () => {
			if (!editor || !toolbarRef.current) return;

			const { state, view } = editor;

			if (!view.hasFocus()) return;

			const { from } = state.selection;
			const cursorCoords = view.coordsAtPos(from);

			if (windowSize.height < rect.height) {
				if (cursorCoords && toolbarRef.current) {
					const toolbarHeight = toolbarRef.current.getBoundingClientRect().height;
					const isEnoughSpace = windowSize.height - cursorCoords.top - toolbarHeight > 0;

					// If not enough space, scroll until the cursor is the middle of the screen
					if (!isEnoughSpace) {
						const scrollY = cursorCoords.top - windowSize.height / 2 + toolbarHeight;
						window.scrollTo({
							top: scrollY,
							behavior: 'smooth'
						});
					}
				}
			}
		};

		checkCursorVisibility();
	}, [editor, rect.height, windowSize.height]);

	React.useEffect(() => {
		if (!isMobile && mobileView !== 'main') {
			setMobileView('main');
		}
	}, [isMobile, mobileView]);

	return (
		<EditorContext.Provider value={{ editor }}>
			<div className={`tiptap-editor-container ${error ? 'error' : ''} ${className}`}>
				<Toolbar
					ref={toolbarRef}
					style={
						isMobile
							? {
									bottom: `calc(100% - ${windowSize.height - rect.y}px)`
								}
							: {}
					}
				>
					{mobileView === 'main' ? (
						<MainToolbarContent
							onHighlighterClick={() => setMobileView('highlighter')}
							onLinkClick={() => setMobileView('link')}
							isMobile={isMobile}
						/>
					) : (
						<MobileToolbarContent
							type={mobileView === 'highlighter' ? 'highlighter' : 'link'}
							onBack={() => setMobileView('main')}
						/>
					)}
				</Toolbar>

				<div className="content-wrapper">
					<EditorContent
						editor={editor}
						role="presentation"
						className="simple-editor-content"
					/>
				</div>
				{error && <div className="tiptap-error-message">{error}</div>}
			</div>
		</EditorContext.Provider>
	);
}
