import {
  DragEvent,
  forwardRef,
  KeyboardEvent,
  memo,
  MouseEvent,
  useEffect,
  useImperativeHandle,
  useRef,
  useState
} from 'react'
import { Editor as TTEditor } from '@tiptap/core'
import { EditorContent } from '@tiptap/react'

import { ActiveEditorComment, BlurAtTopOptions } from '@gitmono/editor'
import { LayeredHotkeys } from '@gitmono/ui'

import { EMPTY_HTML } from '@/atoms/markdown'
import { AttachmentLightbox } from '@/components/AttachmentLightbox'
import { CodeBlockLanguagePicker } from '@/components/CodeBlockLanguagePicker'
import { EditorBubbleMenu } from '@/components/EditorBubbleMenu'
import { MentionInteractivity } from '@/components/InlinePost/MemberHovercard'
import { MentionList } from '@/components/MarkdownEditor/MentionList'
import { ReactionList } from '@/components/MarkdownEditor/ReactionList'
import { ResourceMentionList } from '@/components/MarkdownEditor/ResourceMentionList'
import { DropProps, useEditorFileHandlers } from '@/components/MarkdownEditor/useEditorFileHandlers'
import { HighlightCommentPopover } from '@/components/NoteComments/HighlightCommentPopover'
import { useUploadNoteAttachments } from '@/components/Post/Notes/Attachments/useUploadAttachments'
import { NoteCommentPreview } from '@/components/Post/Notes/CommentRenderer'
import { ADD_ATTACHMENT_SHORTCUT, SlashCommand } from '@/components/Post/Notes/SlashCommand'
import { useSimpleNoteEditor } from '@/components/SimpleNoteEditor/useSimpleNoteEditor'
import { useAutoScroll } from '@/hooks/useAutoScroll'

interface Props {
  commentId: string
  editable?: 'all' | 'viewer'
  autofocus?: boolean
  content: string
  onBlurAtTop?: BlurAtTopOptions['onBlur']
  onKeyDown?: (event: KeyboardEvent) => void
  onChange?: (html: string) => void
}

export interface SimpleNoteContentRef {
  focus(pos: 'start' | 'end' | 'restore' | 'start-newline' | MouseEvent): void
  handleDrop(props: DropProps): void
  handleDragOver(isOver: boolean, event: DragEvent): void
  editor: TTEditor | null
  clearAndBlur(): void
  insertReaction: TTEditor['commands']['insertReaction']
  uploadAndAppendAttachments: (files: File[]) => Promise<void>
  getLinkedIssues(): string[]
}

export const SimpleNoteContent = memo(
  forwardRef<SimpleNoteContentRef, Props>((props, ref) => {
    const { commentId, editable = 'viewer', autofocus = false, onBlurAtTop, content, onChange } = props

    const [activeComment, setActiveComment] = useState<ActiveEditorComment | null>(null)
    const [hoverComment, setHoverComment] = useState<ActiveEditorComment | null>(null)
    const [openAttachmentId, setOpenAttachmentId] = useState<string | undefined>()

    const canUploadAttachments = editable === 'all'
    const upload = useUploadNoteAttachments({ noteId: commentId, enabled: canUploadAttachments })

    const editor = useSimpleNoteEditor({
      content,
      autofocus,
      editable: editable,
      onHoverComment: setHoverComment,
      onActiveComment: setActiveComment,
      onOpenAttachment: setOpenAttachmentId,
      onBlurAtTop
    })

    const { onDrop, onPaste, imperativeHandlers, uploadAndAppendAttachments } = useEditorFileHandlers({
      enabled: canUploadAttachments,
      upload,
      editor
    })

    // these functions allow us to call editorRef?.current?.handleDrop() etc. on the parent container
    useImperativeHandle(
      ref,
      () => ({
        clearAndBlur: () => editor.chain().setContent(EMPTY_HTML).blur().run(),
        insertReaction: (props) => !!editor.commands.insertReaction(props),
        focus: (pos) => {
          if (pos === 'start') {
            editor.commands.focus('start')
          } else if (pos === 'start-newline') {
            editor.commands.focus('start')
            editor.commands.insertContent('\n')
          } else if (pos === 'end') {
            editor.commands.focus('end')
          } else if (pos === 'restore') {
            editor.commands.focus()
          } else if ('clientX' in pos && 'clientY' in pos && 'target' in pos) {
            if (editor.view.dom.contains(pos.target as Node)) {
              return
            }

            const { left, right, top } = editor.view.dom.getBoundingClientRect()
            const isRight = pos.clientX > right
            const editorPos = editor.view.posAtCoords({
              left: isRight ? right : left,
              top: pos.clientY
            })

            if (editorPos) {
              const posAdjustment = isRight && editor.view.coordsAtPos(editorPos.pos).left === left ? -1 : 0

              editor.commands.focus(editorPos.pos + posAdjustment)
            } else if (pos.clientY < top) {
              editor.commands.focus('start')
            } else {
              editor.commands.focus('end')
            }
          }
        },
        getLinkedIssues: () => {
          const issues: string[] = []
          
          editor.state.doc.descendants((node) => {
            if (node.type.name === 'linkIssue' && node.attrs.id) {
              issues.push(node.attrs.id)
            }
          })
        
          return Array.from(new Set(issues))
        },
        uploadAndAppendAttachments,
        ...imperativeHandlers,
        editor
      }),
      [editor, imperativeHandlers, uploadAndAppendAttachments]
    )

    const containerRef = useRef<HTMLDivElement>(null)

    useAutoScroll({
      ref: containerRef,
      enabled: true
    })

    useEffect(() => {
      if (!editor || !onChange) return

      const handleUpdate = () => {
        const html = editor.getHTML()

        onChange?.(html)
      }

      editor.on('update', handleUpdate)

      return () => {
        editor.off('update', handleUpdate)
      }
    }, [editor, onChange])

    return (
      <div ref={containerRef} className='relative mb-2 h-[95%] min-h-[100px] overflow-auto'>
        <LayeredHotkeys
          keys={ADD_ATTACHMENT_SHORTCUT}
          callback={() => {
            if (!editor.isFocused) return

            const input = document.createElement('input')

            input.type = 'file'
            input.onchange = async () => {
              if (input.files?.length) {
                upload({
                  files: Array.from(input.files),
                  editor
                })
              }
            }
            input.click()
          }}
          options={{ enableOnContentEditable: true, enableOnFormTags: true }}
        />

        <NoteCommentPreview
          onExpand={() => {
            if (hoverComment) {
              setHoverComment(null)
              setActiveComment(hoverComment)
            }
          }}
          previewComment={activeComment ? null : hoverComment}
          editor={editor}
          noteId={commentId}
        />
        <MentionInteractivity container={containerRef} />
        <CodeBlockLanguagePicker editor={editor} />
        <SlashCommand editor={editor} upload={upload} />
        <MentionList editor={editor} />
        <ResourceMentionList editor={editor} />
        <ReactionList editor={editor} />

        <AttachmentLightbox
          selectedAttachmentId={openAttachmentId}
          onClose={() => setOpenAttachmentId(undefined)}
          onSelectAttachment={({ id }) => setOpenAttachmentId(id)}
        />

        <HighlightCommentPopover
          activeComment={activeComment}
          editor={editor}
          noteId={commentId}
          onCommentDeactivated={() => setActiveComment(null)}
        />

        <EditorBubbleMenu editor={editor} canComment />

        <EditorContent editor={editor} onKeyDown={props.onKeyDown} onPaste={onPaste} onDrop={onDrop} />
      </div>
    )
  })
)

SimpleNoteContent.displayName = 'SimpleNoteContent'
