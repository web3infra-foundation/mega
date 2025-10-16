import { useMemo, useRef, useState } from "react";
import { ItemInput } from "@primer/react/lib/SelectPanel/types";
import { useAvatars } from "@/components/Issues/utils/sideEffect";
import { ReviewerInfo } from "@gitmono/types";

export const useReviewerSelector = ({
                                      reviewers,
                                      reviewRequest,
                                      avatars
                                    }: {
  reviewers: ReviewerInfo[]
  reviewRequest: (selected: string[]) => void
  avatars: ReturnType<typeof useAvatars>
}) => {
  const initialReviewers = useMemo(() => reviewers.map(item => item.username), [reviewers])
  const [selectedUsers, setSelectedUsers] = useState<string[]>([])
  const shouldFetch = useRef(false)
  const [open, setOpen] = useState(false)

  const handleAssignees = (selected: ItemInput[]) => {
    const newSelection = [...selected.map((i) => i.text).filter((t): t is string => typeof t === 'string')]


    setSelectedUsers(newSelection)
    shouldFetch.current = true

  }

  const handleOpenChange = (open: boolean) => {
    setOpen(open)
    if (!open && shouldFetch.current) {
      // Only submit newly selected reviewers (not existing ones)
      const currentSet = new Set(initialReviewers)
      const newlySelected = selectedUsers.filter(user => !currentSet.has(user))

      // Only make the request if there are new reviewers to add
      if (newlySelected.length > 0) {
        reviewRequest(newlySelected)
      }
      shouldFetch.current = false
    }

  }

  const fetchSelected = useMemo(() => {
    // Only show existing reviewers from backend
    return avatars.filter((user) => initialReviewers.includes(user.text as string))
  }, [avatars, initialReviewers])

  const availableAvatars = useMemo(() => {
    const existingReviewerSet = new Set(initialReviewers)

    return avatars.filter((user) => !existingReviewerSet.has(user.text as string))

  }, [avatars, initialReviewers])

  return {
    open,
    handleAssignees,
    handleOpenChange,
    fetchSelected,
    availableAvatars
  }
}
