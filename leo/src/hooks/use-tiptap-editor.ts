'use client';

import * as React from 'react';
import { Editor, useCurrentEditor } from '@tiptap/react';

export function useTiptapEditor(providedEditor?: Editor | null): Editor | null {
	const { editor: coreEditor } = useCurrentEditor();
	return React.useMemo(() => providedEditor || coreEditor, [providedEditor, coreEditor]);
}
