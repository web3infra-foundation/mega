import { useAtom } from 'jotai'

import { ConversationItem } from '@gitmono/types/generated'
import {
  Button,
  CopyIcon,
  DotsHorizontal,
  EyeHideIcon,
  PencilIcon,
  PreferenceIcon,
  QuoteIcon,
  TrashIcon
} from '@gitmono/ui'
import { DropdownMenu } from '@gitmono/ui/DropdownMenu'
import { buildMenuItems } from '@gitmono/ui/Menu'

import { useDeleteIssueComment } from '@/hooks/issues/useDeleteIssueComment'
import { useDeleteMrCommentDelete } from '@/hooks/useDeleteMrCommentDelete'

import { editIdAtom } from '../Issues/utils/store'
import { SimpleNoteContentRef } from '../SimpleNoteEditor/SimpleNoteContent';
import { generateJSON } from '@tiptap/core';
import toast from 'react-hot-toast';

interface CommentDropdownMenuProps {
  id: string
  Conversation: ConversationItem
  CommentType: 'mr' | 'issue' | (string & {})
  editorRef: React.RefObject<SimpleNoteContentRef>
}


export function CommentDropdownMenu({ Conversation, id, CommentType, editorRef }: CommentDropdownMenuProps) {
  const { mutate: deleteComment } = useDeleteMrCommentDelete(id)
  const { mutate: deleteIssueComment } = useDeleteIssueComment(id)
  const [_editId, setEditId] = useAtom(editIdAtom)
  const commentContent = typeof Conversation.comment === 'string' 
  ? Conversation.comment 
  : String(Conversation.comment)

  const handleDelete = () => {
    switch (CommentType) {
      case 'issue':
        deleteIssueComment(Conversation.id)
        break
      case 'mr':
        deleteComment(Conversation.id)
        break
      default:
        return
    }
  }

  const handleQuote = () => {
    if (!editorRef?.current?.editor || !Conversation.comment) return
    
    try {
      const jsonContent = generateJSON(commentContent, editorRef.current.editor.extensionManager.extensions)
      
      editorRef.current.editor.chain().insertContent([
        {
          type: 'blockquote',
          content: jsonContent.content,
        },
        {
          type: 'paragraph',
          content: []
        }
      ]).run()

      setTimeout(() => {
        const editorDom = editorRef.current?.editor?.view?.dom
        
        if (editorDom) {
          editorDom.scrollIntoView({ behavior: 'smooth', block: 'center' })

          setTimeout(() => {
            editorRef.current?.editor?.commands.focus('end', { scrollIntoView: false })
          }, 150)
        }
      }, 300)

    } catch (error) {
      toast.error('Quote failed, Please try again later.')
    }
  }


    const handleCopy = () => {
    let value = commentContent;

    if (value.includes('<') && value.includes('>')) {
      const tempDiv = document.createElement('div');

      tempDiv.innerHTML = value;
      value = tempDiv.textContent || tempDiv.innerText || '';
    }

    value = value.trim();

    if (!value) return;

    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(value)
      .catch(() => {
        toast.error('Copy failed, Please try again later.')
      })
    } else {
      const textArea = document.createElement('textarea');

      textArea.value = value;
      textArea.style.position = 'fixed';
      textArea.style.left = '-9999px';
      textArea.style.top = '-9999px';
      textArea.style.opacity = '0';
      textArea.style.pointerEvents = 'none';
      textArea.style.zIndex = '-1000';
      document.body.appendChild(textArea);
      
      setTimeout(() => {
        try {
          textArea.focus();
          
          setTimeout(() => {
            textArea.select();
            textArea.setSelectionRange(0, textArea.value.length);
            
            const successful = document.execCommand('copy');
            
            if (!successful) {
              toast.error('Copy failed, Please try again later.')
            }
            
            document.body.removeChild(textArea);
          }, 10);
        } catch (err) {
          document.body.removeChild(textArea);
        }
      }, 10);
    }
  };

  const items = buildMenuItems([
    {
      type: 'item',
      label: 'Copy',
      leftSlot: <CopyIcon />,
      onSelect: () => handleCopy(),
    },
    {
      type: 'item',
      label: 'Quote',
      leftSlot: <QuoteIcon />,
      onSelect: () => handleQuote(),
    },
    {
      type: 'item',
      label: 'Reference',
      leftSlot: <PreferenceIcon />
    },
    { type: 'separator' },
    {
      type: 'item',
      label: 'Edit',
      leftSlot: <PencilIcon />,
      onSelect: () => setEditId(Conversation.id)
    },
    {
      type: 'item',
      label: 'Hide',
      leftSlot: <EyeHideIcon />
    },
    {
      type: 'item',
      label: 'Delete',
      leftSlot: <TrashIcon isAnimated />,
      destructive: true,
      onSelect: () => handleDelete()
    }
  ])

  return (
    <>
      <DropdownMenu
        items={items}
        align='end'
        trigger={<Button variant='plain' iconOnly={<DotsHorizontal />} accessibilityLabel='Comment actions dropdown' />}
      />
    </>
  )
}
