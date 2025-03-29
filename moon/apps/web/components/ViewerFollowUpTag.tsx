import { SubjectFollowUp } from '@gitmono/types/generated'
import { FollowUpTag } from '@gitmono/ui/FollowUpTag'

interface ViewerFollowUpTagProps {
  followUps: SubjectFollowUp[]
}

export function ViewerFollowUpTag({ followUps }: ViewerFollowUpTagProps) {
  const viewerFollowUp = followUps.find((followUp) => followUp.belongs_to_viewer)

  return <FollowUpTag followUpAt={viewerFollowUp?.show_at} />
}
