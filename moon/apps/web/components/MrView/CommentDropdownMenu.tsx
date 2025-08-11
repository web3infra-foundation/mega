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

interface CommentDropdownMenuProps {
  id: string
  Conversation: ConversationItem
  CommentType: 'mr' | 'issue' | (string & {})
}

export function CommentDropdownMenu({ Conversation, id, CommentType }: CommentDropdownMenuProps) {
  const { mutate: deleteComment } = useDeleteMrCommentDelete(id)
  const { mutate: deleteIssueComment } = useDeleteIssueComment(id)
  const [_editId, setEditId] = useAtom(editIdAtom)

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

  const items = buildMenuItems([
    {
      type: 'item',
      label: 'Copy',
      leftSlot: <CopyIcon />
    },
    {
      type: 'item',
      label: 'Quote',
      leftSlot: <QuoteIcon />
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
