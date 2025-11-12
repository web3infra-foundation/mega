import React from 'react'

import { Button, LoadingSpinner } from '@gitmono/ui'
import { PicturePlusIcon, TrashIcon } from '@gitmono/ui/Icons'
import { cn } from '@gitmono/ui/utils'

import { EMPTY_HTML } from '@/atoms/markdown'
import { MergeBox } from '@/components/ClBox/MergeBox'
import TimelineItems from '@/components/ClView/TimelineItems'
import { BadgeItem } from '@/components/Issues/IssueNewPage'
import { splitFun } from '@/components/Issues/utils/sideEffect'
import { WorkWithChatDialog } from '@/components/Issues/WorkWithChatDialog'
import { MemberAvatar } from '@/components/MemberAvatar'
import { ComposerReactionPicker } from '@/components/Reactions/ComposerReactionPicker'
import { SimpleNoteContent } from '@/components/SimpleNoteEditor/SimpleNoteContent'
import { CommonDetailData } from '@/utils/types'

interface ConversationTabProps {
  detailIsLoading: boolean
  ReviewIsLoading: boolean
  clDetail: any
  id: string
  editorRef: any
  reviewers: any[]
  dropzone: any
  handleChange: (html: string) => void
  onKeyDownScrollHandler: any
  isReactionPickerOpen: boolean
  setIsReactionPickerOpen: (open: boolean) => void
  login: boolean
  clCloseIsPending: boolean
  handleClClose: () => void
  closeHint: string
  clReopenIsPending: boolean
  handleClReopen: () => void
  clCommentIsPending: boolean
  send_comment: () => void
  availableAvatars: any[]
  review_open: boolean
  review_handleOpenChange: (open: boolean) => void
  handleReviewers: (selected: any) => void
  review_fetchSelected: any
  memberMap: Map<string, any>
  handleDeleteReviewer: (username: string) => void
  avatars: any[]
  open: boolean
  handleOpenChange: (open: boolean) => void
  handleAssignees: (selected: any) => void
  fetchSelected: any
  labels: any[]
  label_open: boolean
  label_handleOpenChange: (open: boolean) => void
  handleLabels: (selected: any) => void
  label_fetchSelected: any
  labelMap: Map<string, any>
  buttonClasses: string
}

export const ConversationTab = React.memo<ConversationTabProps>(
  ({
    detailIsLoading,
    ReviewIsLoading,
    clDetail,
    id,
    editorRef,
    reviewers,
    dropzone,
    handleChange,
    onKeyDownScrollHandler,
    isReactionPickerOpen,
    setIsReactionPickerOpen,
    login,
    clCloseIsPending,
    handleClClose,
    closeHint,
    clReopenIsPending,
    handleClReopen,
    clCommentIsPending,
    send_comment,
    availableAvatars,
    review_open,
    review_handleOpenChange,
    handleReviewers,
    review_fetchSelected,
    memberMap,
    handleDeleteReviewer,
    avatars,
    open,
    handleOpenChange,
    handleAssignees,
    fetchSelected,
    labels,
    label_open,
    label_handleOpenChange,
    handleLabels,
    label_fetchSelected,
    labelMap,
    buttonClasses
  }) => (
    <div className='flex gap-40'>
      <div className='mt-3 flex w-[60%] flex-col'>
        {detailIsLoading || ReviewIsLoading ? (
          <div className='flex h-16 items-center justify-center'>
            <LoadingSpinner />
          </div>
        ) : (
          clDetail && (
            <TimelineItems
              detail={clDetail as CommonDetailData}
              id={id}
              type='cl'
              editorRef={editorRef}
              reviewers={reviewers}
            />
          )
        )}
        <div style={{ marginTop: '12px' }} className='prose'>
          <div className='w-full'>{clDetail && clDetail.status === 'Open' && <MergeBox prId={id} />}</div>
          <h2 style={{ marginTop: '15px', marginBottom: '15px' }}>Add a comment</h2>
          <input {...dropzone.getInputProps()} />
          <div className='rounded-lg border p-6' style={{ marginTop: '15px', marginBottom: '15px' }}>
            <SimpleNoteContent
              commentId='temp'
              ref={editorRef}
              editable='all'
              content={EMPTY_HTML}
              onKeyDown={onKeyDownScrollHandler}
              onChange={(html) => handleChange(html)}
            />
            <Button
              variant='plain'
              iconOnly={<PicturePlusIcon />}
              accessibilityLabel='Add files'
              onClick={dropzone.open}
              tooltip='Add files'
            />
            <ComposerReactionPicker
              editorRef={editorRef}
              open={isReactionPickerOpen}
              onOpenChange={setIsReactionPickerOpen}
            />
          </div>
          <div className='flex justify-end gap-2'>
            {clDetail && clDetail.status === 'Open' && (
              <Button
                disabled={!login || clCloseIsPending}
                onClick={handleClClose}
                aria-label='Close Change List'
                className={cn(buttonClasses)}
                loading={clCloseIsPending}
              >
                {closeHint}
              </Button>
            )}
            {clDetail && clDetail.status === 'Closed' && (
              <Button
                disabled={!login || clReopenIsPending}
                onClick={handleClReopen}
                aria-label='Reopen Change List'
                className={cn(buttonClasses)}
                loading={clReopenIsPending}
              >
                Reopen Change List
              </Button>
            )}
            <Button
              disabled={!login || clCommentIsPending}
              onClick={() => send_comment()}
              aria-label='Comment'
              className={cn(buttonClasses)}
              loading={clCommentIsPending}
            >
              Comment
            </Button>
          </div>
        </div>
      </div>
      <div className='flex flex-1 flex-col flex-wrap items-center'>
        <BadgeItem
          selectPannelProps={{ title: 'Assign up to 10 people to this issue' }}
          items={availableAvatars}
          title='Reviewers'
          open={review_open}
          onOpenChange={(open) => review_handleOpenChange(open)}
          handleGroup={(selected) => handleReviewers(selected)}
          selected={review_fetchSelected}
        >
          {(el) => {
            const names = Array.from(new Set(splitFun(el)))

            return (
              <div className='pointer-events-none'>
                {names.map((i) => {
                  const reviewer = reviewers.find((r) => r.username === i)
                  const isApproved = reviewer?.approved ?? false

                  return (
                    <div key={i} className='mb-4 flex items-center gap-2 px-4 text-sm text-gray-500'>
                      <MemberAvatar size='sm' member={memberMap.get(i)} />
                      <span className='flex-1'>{i}</span>
                      <span
                        className={`ml-2 inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${
                          isApproved ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
                        }`}
                      >
                        {isApproved ? 'Approved' : 'Pending'}
                      </span>
                      <span
                        onClick={(e) => {
                          e.stopPropagation()
                          handleDeleteReviewer(i)
                        }}
                        className='pointer-events-auto cursor-pointer rounded-full border-2 hover:bg-red-800'
                      >
                        <TrashIcon />
                      </span>
                    </div>
                  )
                })}
              </div>
            )
          }}
        </BadgeItem>
        <BadgeItem
          selectPannelProps={{ title: 'Assign up to 10 people to this issue' }}
          items={avatars}
          title='Assignees'
          handleGroup={(selected) => handleAssignees(selected)}
          open={open}
          onOpenChange={(open) => handleOpenChange(open)}
          selected={fetchSelected}
        >
          {(el) => {
            const names = Array.from(new Set(splitFun(el)))

            return (
              <>
                {names.map((i) => (
                  <div key={i} className='mb-4 flex items-center gap-2 px-4 text-sm text-gray-500'>
                    <MemberAvatar size='sm' member={memberMap.get(i)} />
                    <span>{i}</span>
                  </div>
                ))}
              </>
            )
          }}
        </BadgeItem>
        <BadgeItem
          selectPannelProps={{ title: 'Apply labels to this issue' }}
          items={labels}
          title='Labels'
          handleGroup={(selected) => handleLabels(selected)}
          open={label_open}
          onOpenChange={(open) => label_handleOpenChange(open)}
          selected={label_fetchSelected}
        >
          {(el) => {
            const names = splitFun(el)

            return (
              <>
                <div className='flex flex-wrap items-start px-4'>
                  {names.map((i) => {
                    const label = labelMap.get(i) ?? {}

                    return (
                      <div key={i} className='mb-4 flex items-center justify-center pr-2'>
                        <div
                          className='rounded-full border px-2 text-sm text-[#fff]'
                          style={{ backgroundColor: label.color, borderColor: label.color }}
                        >
                          {label.name}
                        </div>
                      </div>
                    )
                  })}
                </div>
              </>
            )
          }}
        </BadgeItem>
        <BadgeItem title='Type' items={labels} />
        <BadgeItem title='Projects' items={labels} />
        <BadgeItem title='Milestones' items={labels} />
        <div className='w-full' style={{ marginTop: '24px' }}>
          <WorkWithChatDialog />
        </div>
      </div>
    </div>
  )
)

ConversationTab.displayName = 'ConversationTab'
