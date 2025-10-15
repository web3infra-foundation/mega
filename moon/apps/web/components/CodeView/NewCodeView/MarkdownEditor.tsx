'use client'

import { useEffect, useState } from 'react'
import { EditorContent, useEditor } from '@tiptap/react'
import Markdown from 'react-markdown'

import { StarterKit } from '@gitmono/editor/extensions'

interface MarkdownEditorProps {
  contentState: [string, React.Dispatch<React.SetStateAction<string>>]
  disabled?: boolean
}
export default function MarkdownEditor({ contentState, disabled = false }: MarkdownEditorProps) {
  const [content, setContent] = contentState
  const [lineCount, setLineCount] = useState(1)
  const [isPreview, setIsPreview] = useState(false)

  const textEditor = useEditor({
    extensions: StarterKit(),
    editable: !disabled,
    onUpdate: ({ editor }) => {
      const text = editor.getText().replace(/\n\n/g, '\n')

      setContent(editor.getText())
      setLineCount(text.split('\n').length || 1)
    },
    editorProps: {
      attributes: {
        class: 'max-w-full focus:outline-none font-mono text-sm leading-6'
      }
    }
  })

  // 当 disabled 状态变化时，更新编辑器的可编辑状态
  useEffect(() => {
    if (textEditor) {
      textEditor.setEditable(!disabled)
    }
  }, [disabled, textEditor])

  const lineNumbers = Array.from({ length: lineCount }, (_, i) => i + 1)

  return (
    <div className={`flex h-full w-full flex-col rounded-xl border border-[#bec7ce] ${disabled ? 'opacity-60' : ''}`}>
      {/* Toggle Bar */}
      <div className='flex h-14 w-full items-center rounded-t-xl border border-b-[#d0d9e0] bg-[#f9fbfd] p-4'>
        <div className='inline-flex rounded-md border border-gray-300 bg-white'>
          <button
            onClick={() => setIsPreview(false)}
            disabled={disabled}
            className={`rounded-l-md px-4 py-2 text-sm font-medium ${
              !isPreview ? 'bg-gray-100 text-gray-900' : 'bg-white text-gray-500 hover:text-gray-700'
            } ${disabled ? 'cursor-not-allowed opacity-50' : ''}`}
          >
            Edit
          </button>
          <button
            onClick={() => setIsPreview(true)}
            disabled={disabled}
            className={`rounded-r-md px-4 py-2 text-sm font-medium ${
              isPreview ? 'bg-gray-100 text-gray-900' : 'bg-white text-gray-500 hover:text-gray-700'
            } ${disabled ? 'cursor-not-allowed opacity-50' : ''}`}
          >
            Preview
          </button>
        </div>
        {disabled && (
          <span className='ml-4 text-sm text-gray-500 italic'>
            Folders don't need content
          </span>
        )}
      </div>

      {/* Content Area */}
      <div className={`flex flex-1 overflow-x-auto ${disabled ? 'bg-gray-100' : ''}`}>
        {isPreview ? (
          <div className='prose h-full w-full max-w-none overflow-y-auto px-8 pb-4 pt-6'>
            <Markdown>{content}</Markdown>
          </div>
        ) : (
          <div className='flex w-full font-mono text-sm leading-6'>
            <div className='select-none rounded-l-xl border-r border-gray-200 bg-gray-50 px-4 text-right text-gray-400'>
              {lineNumbers.map((n) => (
                <div key={n}>{n}</div>
              ))}
            </div>
            <div className={`flex-1 pl-4 ${disabled ? 'cursor-not-allowed' : ''}`}>
              <EditorContent editor={textEditor} />
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
