import { Button, CopyIcon, DotsHorizontal, PreferenceIcon, PencilIcon, QuoteIcon, TrashIcon, EyeHideIcon } from '@gitmono/ui'
import { DropdownMenu } from '@gitmono/ui/DropdownMenu'
import { buildMenuItems } from '@gitmono/ui/Menu'
import { ConversationItem } from '@gitmono/types/generated';
import { useDeleteIssueComment } from '@/hooks/issues/useDeleteIssueComment'
import { useDeleteMrCommentDelete } from '@/hooks/useDeleteMrCommentDelete'

interface CommentDropdownMenuProps {
  id: string
  Conversation: ConversationItem
  CommentType: 'mr' | 'issue' | (string & {})
}

export function CommentDropdownMenu({ Conversation, id, CommentType }: CommentDropdownMenuProps) {
  const { mutate: deleteComment } = useDeleteMrCommentDelete(id)
  const { mutate: deleteIssueComment } = useDeleteIssueComment(id)

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
      leftSlot: <CopyIcon />,
    },
    {
      type: 'item',
      label: 'Quote',
      leftSlot: <QuoteIcon />,
    },
    {
      type: 'item',
      label: 'Reference',
      leftSlot: <PreferenceIcon />,
    },
    { type: 'separator' },
    {
      type: 'item',
      label: 'Edit',
      leftSlot: <PencilIcon />,
    },
    {
      type: 'item',
      label: 'Hide',
      leftSlot: <EyeHideIcon />,
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
