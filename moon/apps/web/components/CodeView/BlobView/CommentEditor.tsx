import React, { useState } from 'react'
import { useEditor, EditorContent } from '@tiptap/react'
import { getMarkdownExtensions } from '@gitmono/editor/markdown'
import { Button } from '@gitmono/ui/Button'

interface CommentEditorProps {
  onSubmit: (content: string) => void
  onCancel?: () => void
  placeholder?: string
  initialContent?: string
  isSubmitting?: boolean
}

export function CommentEditor({
                                onSubmit,
                                onCancel,
                                placeholder = '写下您的评论...',
                                initialContent = '',
                                isSubmitting = false
                              }: CommentEditorProps) {
  const [content, setContent] = useState(initialContent)

  const editor = useEditor({
    extensions: getMarkdownExtensions({
      placeholder,
      enableInlineAttachments: true,
      blurAtTop: { enabled: false },
      codeBlockHighlighted: {
        HTMLAttributes: {
          class: 'hljs'
        }
      }
    }),
    content: initialContent,
    onUpdate: ({ editor }) => {
      setContent(editor.getHTML())
    },
    editorProps: {
      attributes: {
        class: 'prose prose-sm max-w-none focus:outline-none min-h-[80px] p-3 border rounded-md'
      }
    }
  })

  const handleSubmit = () => {
    if (!editor) return

    const markdownContent = editor.getText().trim()

    if (markdownContent) {
      onSubmit(content)
      editor.commands.clearContent()
      setContent('')
    }
  }

  const handleKeyDown = (event: React.KeyboardEvent) => {
    // Ctrl/Cmd + Enter 快速提交
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
      event.preventDefault()
      handleSubmit()
    }
  }

  return (
    <div className="border rounded-lg bg-white shadow-sm">
      <div className="relative">
        <EditorContent
          editor={editor}
          onKeyDown={handleKeyDown}
          className="min-h-[80px]"
        />
      </div>

      <div className="flex items-center justify-between p-3 border-t bg-gray-50">
        <div className="flex items-center space-x-2 text-xs text-gray-500">
          <span>支持 Markdown 语法</span>
          <span>•</span>
          <span>`Ctrl` + `Enter` 快速发布</span>
        </div>

        <div className="flex items-center space-x-2">
          {onCancel && (
            <Button
              variant="text"
              size="sm"
              onClick={onCancel}
              disabled={isSubmitting}
            >
              取消
            </Button>
          )}
          <Button
            variant="primary"
            size="sm"
            onClick={handleSubmit}
            disabled={!content.trim() || isSubmitting}
            loading={isSubmitting}
          >
            发布评论
          </Button>
        </div>
      </div>
    </div>
  )
}