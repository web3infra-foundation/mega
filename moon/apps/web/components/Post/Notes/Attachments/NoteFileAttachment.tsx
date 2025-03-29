import { Attachment } from '@gitmono/types'
import { Button, TrashIcon } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

import { FileAttachment } from '@/components/FileAttachment'

interface Props {
  attachment: Attachment
  isUploading: boolean
  editable: boolean
  onDelete?: () => void
}

export function NoteFileAttachment(props: Props) {
  const { attachment, isUploading, editable, onDelete } = props

  return (
    <div
      className={cn('relative w-full overflow-hidden rounded', {
        'cursor-grab': editable,
        'cursor-auto': !editable
      })}
    >
      <div
        className={cn('relative flex w-full flex-row items-center justify-between gap-2 rounded border', {
          'opacity-30': isUploading
        })}
      >
        <FileAttachment
          showActions
          attachment={attachment}
          extraActions={
            editable &&
            onDelete &&
            !isUploading && (
              <Button
                iconOnly={<TrashIcon size={20} />}
                variant='plain'
                accessibilityLabel='Delete attachment'
                onClick={onDelete}
              />
            )
          }
        />
      </div>
    </div>
  )
}
