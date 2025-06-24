import { Button, FaceSmilePlusIcon } from '@gitmono/ui'

import { MarkdownEditorRef } from '../MarkdownEditor'
import { ReactionPicker } from './ReactionPicker'
import { SimpleNoteContentRef } from '../SimpleNoteEditor/SimpleNoteContent'

interface ComposerReactionPickerProps {
  open?: boolean
  onOpenChange?: (value: boolean) => void
  editorRef: React.RefObject<MarkdownEditorRef> | React.RefObject<SimpleNoteContentRef>
  disabled?: boolean
}

export function ComposerReactionPicker({ open, onOpenChange, editorRef, disabled }: ComposerReactionPickerProps) {
  return (
    <ReactionPicker
      open={open}
      onOpenChange={onOpenChange}
      custom
      trigger={
        <Button
          variant='plain'
          type='button'
          iconOnly={<FaceSmilePlusIcon />}
          accessibilityLabel='Add emoji'
          tooltip='Add emoji'
          tooltipShortcut=':'
          disabled={disabled}
        />
      }
      onReactionSelect={(emoji) => editorRef.current?.insertReaction(emoji)}
      onClose={() => editorRef.current?.focus('restore')}
    />
  )
}
