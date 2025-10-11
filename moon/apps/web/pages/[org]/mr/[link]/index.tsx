'use client'

import React, { useEffect, useRef, useState } from 'react'
import { BaseStyles, ThemeProvider } from '@primer/react'
import { useAtom } from 'jotai'
import dynamic from 'next/dynamic'
import { useRouter } from 'next/router'
import { toast } from 'react-hot-toast'

import { CommonResultMRDetailRes } from '@gitmono/types/generated'
import { Button, LoadingSpinner } from '@gitmono/ui'
import { PicturePlusIcon } from '@gitmono/ui/Icons'
import { cn } from '@gitmono/ui/utils'

import { EMPTY_HTML } from '@/atoms/markdown'
import FileDiff from '@/components/DiffView/FileDiff'
import { BadgeItem } from '@/components/Issues/IssueNewPage'
import TitleInput from '@/components/Issues/TitleInput'
import {
  splitFun,
  useAssigneesSelector,
  useAvatars,
  useChange,
  useLabelMap,
  useLabels,
  useLabelsSelector,
  useMemberMap
} from '@/components/Issues/utils/sideEffect'
import { useReviewerSelector } from '@/components/MrView/useReviewerSelector'
import { editIdAtom, FALSE_EDIT_VAL, mridAtom, refreshAtom } from '@/components/Issues/utils/store'
import { AppLayout } from '@/components/Layout/AppLayout'
import { TabLayout } from '@/components/Layout/TabLayout'
import { MemberAvatar } from '@/components/MemberAvatar'
import { MergeBox } from '@/components/MrBox/MergeBox'
import { tabAtom } from '@/components/MrView/components/Checks/cpns/store'
import TimelineItems from '@/components/MrView/TimelineItems'
import { useHandleBottomScrollOffset } from '@/components/NoteEditor/useHandleBottomScrollOffset'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { ComposerReactionPicker } from '@/components/Reactions/ComposerReactionPicker'
import { SimpleNoteContent, SimpleNoteContentRef } from '@/components/SimpleNoteEditor/SimpleNoteContent'
import { useScope } from '@/contexts/scope'
import { usePostMRAssignees } from '@/hooks/issues/usePostMRAssignees'
import { useGetMrDetail } from '@/hooks/useGetMrDetail'
import { usePostMrClose } from '@/hooks/usePostMrClose'
import { usePostMrComment } from '@/hooks/usePostMrComment'
import { usePostMRLabels } from '@/hooks/usePostMRLabels'
// import { usePostMrMerge } from '@/hooks/usePostMrMerge'
import { usePostMrReopen } from '@/hooks/usePostMrReopen'
import { useUploadHelpers } from '@/hooks/useUploadHelpers'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { trimHtml } from '@/utils/trimHtml'
import { PageWithLayout, CommonDetailData } from '@/utils/types'
import { useGetMrReviewers } from "@/hooks/useGetMrReviewers";
import { usePostMrReviewers } from "@/hooks/usePostMrReviewers";
import { TrashIcon } from "@gitmono/ui/Icons";
import { useDeleteMrReviewers } from "@/hooks/useDeleteMrReviewers";
import { WorkWithChatDialog } from "@/components/Issues/WorkWithChatDialog";

const MRDetailPage: PageWithLayout<any> = () => {
  const router = useRouter()
  const { link: tempId } = router.query
  const [item_id] = useAtom(mridAtom)
  const { scope } = useScope()
  const [login, _setLogin] = useState(true)
  const [isReactionPickerOpen, setIsReactionPickerOpen] = useState(false)
  const id = typeof tempId === 'string' ? tempId : ''
  const { data: MrDetailData, isLoading: detailIsLoading, refetch } = useGetMrDetail(id)
  const { reviewers, isLoading: ReviewIsLoading } = useGetMrReviewers(id)
  const mrDetail = MrDetailData?.data as CommonResultMRDetailRes['data'] | undefined
  const { closeHint, needComment, handleChange } = useChange({ title: 'Close Merge Request' })
  const { mutate: mrAssignees } = usePostMRAssignees()
  const { mutate: mrReviewers } = usePostMrReviewers()
  const { mutate: mrLabels } = usePostMRLabels()
  const Checks = dynamic(() => import('@/components/MrView/components/Checks'))

  const [_, setEditId] = useAtom(editIdAtom)
  const [refresh, setRefresh] = useAtom(refreshAtom)

  useEffect(() => {
    const load = async () => {
      await refetch()
      setEditId(FALSE_EDIT_VAL)
      setRefresh(0)
    }

    load()

  }, [refresh, refetch, setEditId, setRefresh])

  const { mutate: closeMr, isPending: mrCloseIsPending } = usePostMrClose(id)
  const handleMrClose = () => {
    if (closeHint === 'Close with comment') {
      send_comment()
    }
    closeMr(undefined, {
      onSuccess: () => {
        router.push(`/${scope}/mr`)
      }
    })
  }

  const { mutate: reopenMr, isPending: mrReopenIsPending } = usePostMrReopen(id)
  const handleMrReopen = () => {
    if (needComment.current) {
      send_comment()
      needComment.current = false
    }
    reopenMr(undefined, {
      onSuccess: () => {
        router.push(`/${scope}/mr`)
      }
    })
  }

  const { mutate: postMrComment, isPending: mrCommentIsPending } = usePostMrComment(id)

  const send_comment = () => {
    const currentContentHTML = editorRef.current?.editor?.getHTML() ?? '<p></p>'
    const issues = editorRef.current?.getLinkedIssues() || []

    /* eslint-disable-next-line no-console */
    console.log('commentIssues:', issues)

    if (trimHtml(currentContentHTML) === '') {
      toast.error('Please enter the content.')
    } else {
      postMrComment(
        { content: currentContentHTML },
        {
          onSuccess: () => {
            editorRef.current?.clearAndBlur()
          }
        }
      )
    }

  }

  const buttonClasses = 'cursor-pointer'
  const editorRef = useRef<SimpleNoteContentRef>(null)
  const onKeyDownScrollHandler = useHandleBottomScrollOffset({
    editor: editorRef.current?.editor
  })
  const { dropzone } = useUploadHelpers({
    upload: editorRef.current?.uploadAndAppendAttachments
  })

  const avatars = useAvatars()

  const memberMap = useMemberMap()

  const labels = useLabels()

  const labelMap = useLabelMap()

  const { open, handleAssignees, handleOpenChange, fetchSelected } = useAssigneesSelector({
    assignees: MrDetailData?.data?.assignees ?? [],
    assignRequest: (selected) =>
      mrAssignees(
        {
          data: {
            link: id,
            item_id: Number(item_id),
            assignees: selected
          }
        },
        {
          onSuccess: async () => {
            editorRef.current?.clearAndBlur()
            await refetch({ throwOnError: true })
          },
          onError: apiErrorToast
        }
      ),
    avatars
  })
  const {
    open: review_open,
    handleAssignees: handleReviewers,
    handleOpenChange: review_handleOpenChange,
    fetchSelected: review_fetchSelected,
    availableAvatars
  } = useReviewerSelector({
    reviewers,
    reviewRequest: (selected) =>
      mrReviewers(
        {
          link: id,
          data: {
            reviewer_usernames: selected
          }
        },
        {
          onSuccess: async () => {
            editorRef.current?.clearAndBlur()
          },
          onError: apiErrorToast
        }
      ),
    avatars
  })

  const {
    open: label_open,
    handleLabels,
    handleOpenChange: label_handleOpenChange,
    fetchSelected: label_fetchSelected
  } = useLabelsSelector({
    labelList: labels,
    labels: MrDetailData?.data?.labels ?? [],
    updateLabelsRequest: (selected) => {
      mrLabels(
        {
          data: {
            item_id: Number(item_id),
            label_ids: selected,
            link: `${tempId}` 
    }
    },
      {
        onSuccess: async () => {
          editorRef.current?.clearAndBlur()
          await refetch({ throwOnError: true })
        },
          onError: apiErrorToast
      }
    )
    }
  })

  const { mutate: deleteReviewer } = useDeleteMrReviewers()
  const handleDeleteReviewer = (user_name: string) => {
    deleteReviewer({
      link: id,
      data: {
        reviewer_usernames: [user_name]
      }
    })
  }

  const [tab] = useAtom(tabAtom)
  const Conversation = () => (
    <div className='flex gap-40'>
      <div className='mt-3 flex w-[60%] flex-col'>
        {detailIsLoading && ReviewIsLoading ? (
          <div className='flex h-16 items-center justify-center'>
            <LoadingSpinner />
          </div>
        ) : (
          mrDetail && <TimelineItems detail={mrDetail as CommonDetailData} id={id} type='mr' editorRef={editorRef} />
        )}
        <div style={{ marginTop: '12px' }} className='prose'>
          <div className='w-full'>{mrDetail && mrDetail.status === 'Open' && <MergeBox prId={id} />}</div>
          <h2>Add a comment</h2>
          <input {...dropzone.getInputProps()} />
          <div className='rounded-lg border p-6'>
            <SimpleNoteContent
              commentId='temp' //  Temporary filling, replacement later
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
            {mrDetail && mrDetail.status === 'Open' && (
              <Button
                disabled={!login || mrCloseIsPending}
                onClick={handleMrClose}
                aria-label='Close Merge Request'
                className={cn(buttonClasses)}
                loading={mrCloseIsPending}
              >
                {closeHint}
              </Button>
            )}
            {mrDetail && mrDetail.status === 'Closed' && (
              <Button
                disabled={!login || mrReopenIsPending}
                onClick={handleMrReopen}
                aria-label='Reopen Merge Request'
                className={cn(buttonClasses)}
                loading={mrReopenIsPending}
              >
                Reopen Merge Request
              </Button>
            )}
            <Button
              disabled={!login || mrCommentIsPending}
              onClick={() => send_comment()}
              aria-label='Comment'
              className={cn(buttonClasses)}
              loading={mrCommentIsPending}
            >
              Comment
            </Button>
          </div>
        </div>
      </div>
      {/* <SideBar /> */}
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
              <div className={`pointer-events-none`}>
                {names.map((i, index) => {
                  const reviewer = reviewers.find(r => r.username === i)
                  const isApproved = reviewer?.approved ?? false

                  return (
                    // eslint-disable-next-line react/no-array-index-key
                    <div key={ index } className='mb-4 flex items-center gap-2 px-4 text-sm text-gray-500'>
                      <MemberAvatar size='sm' member={ memberMap.get(i) }/>
                      <span className={'flex-1'} >{ i }</span>
                      <span
                        className={ `ml-2 inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${
                          isApproved
                            ? 'bg-green-100 text-green-800'
                            : 'bg-red-100 text-red-800'
                        }` }
                      >
                        { isApproved? 'Approved' : 'Pending' }
                      </span>
                      <span
                        onClick={(e) => {
                          e.stopPropagation()
                          handleDeleteReviewer(i)
                        }}
                        className='pointer-events-auto cursor-pointer border-2 rounded-full hover:bg-red-800'
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
          // eslint-disable-next-line react-hooks/rules-of-hooks
          onOpenChange={(open) => handleOpenChange(open)}
          selected={fetchSelected}
        >
          {(el) => {
            const names = Array.from(new Set(splitFun(el)))

            return (
              <>
                {names.map((i, index) => (
                  // eslint-disable-next-line react/no-array-index-key
                  <div key={index} className='mb-4 flex items-center gap-2 px-4 text-sm text-gray-500'>
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
                <div className='flex flex-wrap items-start px-4'>@
                  {names.map((i, index) => {
                    const label = labelMap.get(i) ?? {}

                    return (
                      // eslint-disable-next-line react/no-array-index-key
                      <div key={index} className='mb-4 flex items-center justify-center pr-2'>
                        <div
                          className='rounded-full border px-2 text-sm text-[#fff]'
                          //eslint-disable-next-line react/forbid-dom-props
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
        <div className='mt-6 w-full'>
          <WorkWithChatDialog />
        </div>
      </div>
    </div>

  )
  const FileChange = () => (
    <>
      <FileDiff id={id} />
    </>
  )

  return (
    <ThemeProvider>
      <BaseStyles>
        <div className='h-screen overflow-auto p-6'>
          {mrDetail && (
            <TitleInput title={mrDetail.title} whoami='mr' id={id} callback={() => refetch({ throwOnError: true })} />
          )}
          <div>
            <TabLayout>
              {tab === 'check' && <Checks mr={item_id} />}
              {tab === 'conversation' && <Conversation />}
              {tab === 'filechange' && <FileChange />}
            </TabLayout>
          </div>
        </div>
      </BaseStyles>
    </ThemeProvider>
  )
}

MRDetailPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default MRDetailPage
