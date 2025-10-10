import { Note } from '@gitmono/types/generated'
import * as Tabs from '@radix-ui/react-tabs'

import { ShareTab } from '@/components/NoteSharePopover/ShareTab'
import { PostComposerType, usePostComposer } from '@/components/PostComposer'

interface NoteShareContentProps {
  note: Note
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function NoteShareContent({ note, open, onOpenChange }: NoteShareContentProps) {
  const { showPostComposer } = usePostComposer()

  return (
    <Tabs.Root defaultValue='share'>
      <ShareTab
        note={note}
        open={open}
        onOpenChange={onOpenChange}
        onCompose={() => showPostComposer({ type: PostComposerType.DraftFromNote, note })}
      />
    </Tabs.Root>
  )
}