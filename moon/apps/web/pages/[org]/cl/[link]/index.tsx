'use client'

import React, { useEffect, useRef, useState } from 'react'
import { BaseStyles, ThemeProvider } from '@primer/react'
import { useAtom } from 'jotai'
import dynamic from 'next/dynamic'
import { useRouter } from 'next/router'
import { toast } from 'react-hot-toast'

import { CommonResultCLDetailRes } from '@gitmono/types/generated'

import TitleInput from '@/components/Issues/TitleInput'
import {
  useAssigneesSelector,
  useAvatars,
  useChange,
  useLabelMap,
  useLabels,
  useLabelsSelector,
  useMemberMap
} from '@/components/Issues/utils/sideEffect'
import { useReviewerSelector } from '@/components/ClView/useReviewerSelector'
import { editIdAtom, FALSE_EDIT_VAL, clidAtom, refreshAtom } from '@/components/Issues/utils/store'
import { AppLayout } from '@/components/Layout/AppLayout'
import { TabLayout } from '@/components/Layout/TabLayout'
import { tabAtom } from '@/components/ClView/components/Checks/cpns/store'
import { useHandleBottomScrollOffset } from '@/components/NoteEditor/useHandleBottomScrollOffset'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { SimpleNoteContentRef } from '@/components/SimpleNoteEditor/SimpleNoteContent'
import { useScope } from '@/contexts/scope'
import { usePostCLAssignees } from '@/hooks/CL/usePostCLAssignees'
import { useGetClDetail } from '@/hooks/CL/useGetClDetail'
import { usePostClClose } from '@/hooks/CL/usePostClClose'
import { usePostClComment } from '@/hooks/CL/usePostClComment'
import { usePostCLLabels } from '@/hooks/CL/usePostCLLabels'
import { usePostClReopen } from '@/hooks/CL/usePostClReopen'
import { useUploadHelpers } from '@/hooks/useUploadHelpers'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { trimHtml } from '@/utils/trimHtml'
import { PageWithLayout } from '@/utils/types'
import { useGetClReviewers } from '@/hooks/CL/useGetClReviewers'
import { usePostClReviewers } from '@/hooks/CL/usePostClReviewers'
import { useDeleteClReviewers } from '@/hooks/CL/useDeleteClReviewers'
import { ConversationTab } from '@/components/ClView/ConversationTab'
import { FileChangeTab } from '@/components/ClView/FileChangeTab'

const CLDetailPage: PageWithLayout<any> = () => {
  const router = useRouter()
  const { link: tempId } = router.query
  const [item_id] = useAtom(clidAtom)
  const { scope } = useScope()
  const [login, _setLogin] = useState(true)
  const [isReactionPickerOpen, setIsReactionPickerOpen] = useState(false)
  const id = typeof tempId === 'string' ? tempId : ''
  const { data: ClDetailData, isLoading: detailIsLoading, refetch } = useGetClDetail(id)
  const { reviewers, isLoading: ReviewIsLoading } = useGetClReviewers(id)
  const clDetail = ClDetailData?.data as CommonResultCLDetailRes['data'] | undefined
  const { closeHint, needComment, handleChange } = useChange({ title: 'Close Change List' })
  const { mutate: clAssignees } = usePostCLAssignees()
  const { mutate: clReviewers } = usePostClReviewers()
  const { mutate: clLabels } = usePostCLLabels()
  const Checks = dynamic(() => import('@/components/ClView/components/Checks'))

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

  const { mutate: closeCl, isPending: clCloseIsPending } = usePostClClose(id)
  const handleClClose = () => {
    if (closeHint === 'Close with comment') {
      send_comment()
    }
    closeCl(undefined, {
      onSuccess: () => {
        router.push(`/${scope}/cl`)
      }
    })
  }

  const { mutate: reopenCl, isPending: clReopenIsPending } = usePostClReopen(id)
  const handleClReopen = () => {
    if (needComment.current) {
      send_comment()
      needComment.current = false
    }
    reopenCl(undefined, {
      onSuccess: () => {
        router.push(`/${scope}/cl`)
      }
    })
  }

  const { mutate: postClComment, isPending: clCommentIsPending } = usePostClComment(id)

  const send_comment = () => {
    const currentContentHTML = editorRef.current?.editor?.getHTML() ?? '<p></p>'
    const issues = editorRef.current?.getLinkedIssues() || []

    /* eslint-disable-next-line no-console */
    console.log('commentIssues:', issues)

    if (trimHtml(currentContentHTML) === '') {
      toast.error('Please enter the content.')
    } else {
      postClComment(
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
    assignees: ClDetailData?.data?.assignees ?? [],
    assignRequest: (selected) =>
      clAssignees(
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
      clReviewers(
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
    labels: ClDetailData?.data?.labels ?? [],
    updateLabelsRequest: (selected) => {
      clLabels(
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

  const { mutate: deleteReviewer } = useDeleteClReviewers()
  const handleDeleteReviewer = (user_name: string) => {
    deleteReviewer({
      link: id,
      data: {
        reviewer_usernames: [user_name]
      }
    })
  }

  const [tab] = useAtom(tabAtom)

  return (
    <ThemeProvider>
      <BaseStyles>
        <div className='h-screen overflow-auto p-6'>
          {clDetail && (
            <TitleInput title={clDetail.title} whoami='cl' id={id} callback={() => refetch({ throwOnError: true })} />
          )}
          <div>
            <TabLayout>
              {tab === 'check' && <Checks cl={item_id} />}
              {tab === 'conversation' && (
                <ConversationTab
                  detailIsLoading={detailIsLoading}
                  ReviewIsLoading={ReviewIsLoading}
                  clDetail={clDetail}
                  id={id}
                  editorRef={editorRef}
                  reviewers={reviewers}
                  dropzone={dropzone}
                  handleChange={handleChange}
                  onKeyDownScrollHandler={onKeyDownScrollHandler}
                  isReactionPickerOpen={isReactionPickerOpen}
                  setIsReactionPickerOpen={setIsReactionPickerOpen}
                  login={login}
                  clCloseIsPending={clCloseIsPending}
                  handleClClose={handleClClose}
                  closeHint={closeHint}
                  clReopenIsPending={clReopenIsPending}
                  handleClReopen={handleClReopen}
                  clCommentIsPending={clCommentIsPending}
                  send_comment={send_comment}
                  availableAvatars={availableAvatars}
                  review_open={review_open}
                  review_handleOpenChange={review_handleOpenChange}
                  handleReviewers={handleReviewers}
                  review_fetchSelected={review_fetchSelected}
                  memberMap={memberMap}
                  handleDeleteReviewer={handleDeleteReviewer}
                  avatars={avatars}
                  open={open}
                  handleOpenChange={handleOpenChange}
                  handleAssignees={handleAssignees}
                  fetchSelected={fetchSelected}
                  labels={labels}
                  label_open={label_open}
                  label_handleOpenChange={label_handleOpenChange}
                  handleLabels={handleLabels}
                  label_fetchSelected={label_fetchSelected}
                  labelMap={labelMap}
                  buttonClasses={buttonClasses}
                />
              )}
              {tab === 'filechange' && <FileChangeTab id={id} />}
            </TabLayout>
          </div>
        </div>
      </BaseStyles>
    </ThemeProvider>
  )
}

CLDetailPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default CLDetailPage
