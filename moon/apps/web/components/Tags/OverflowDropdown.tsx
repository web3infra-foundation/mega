import { useState } from 'react'
import { useRouter } from 'next/router'
import toast from 'react-hot-toast'

import { Tag } from '@gitmono/types'
import { Button, DotsHorizontal, LinkIcon, TrashIcon } from '@gitmono/ui'
import { DropdownMenu } from '@gitmono/ui/DropdownMenu'
import { buildMenuItems } from '@gitmono/ui/Menu'
import { useCopyToClipboard } from '@gitmono/ui/src/hooks'

import { DeleteTagDialog } from './DeleteDialog'

interface Props {
  tag: Tag
}

export function TagOverflowDropdown({ tag }: Props) {
  const router = useRouter()
  const isTagView = router.pathname === '/[org]/tags/[tagName]'
  const [deleteDialogIsOpen, setDeleteDialogIsOpen] = useState(false)
  const [copy] = useCopyToClipboard()

  if (!tag.viewer_can_destroy) return null

  return (
    <>
      <DeleteTagDialog tag={tag} open={deleteDialogIsOpen} onOpenChange={setDeleteDialogIsOpen} />

      <DropdownMenu
        trigger={<Button variant='plain' iconOnly={<DotsHorizontal />} accessibilityLabel='Open menu' />}
        align='end'
        items={buildMenuItems([
          tag.url && {
            type: 'item',
            leftSlot: <LinkIcon />,
            label: 'Copy link',
            kbd: isTagView ? 'mod+shift+c' : undefined,
            onSelect: (): void => {
              tag.url && copy(tag.url)
              toast('Copied to clipboard')
            }
          },
          {
            type: 'item',
            leftSlot: <TrashIcon isAnimated />,
            label: 'Delete',
            destructive: true,
            onSelect: () => setDeleteDialogIsOpen(true)
          }
        ])}
      />
    </>
  )
}
