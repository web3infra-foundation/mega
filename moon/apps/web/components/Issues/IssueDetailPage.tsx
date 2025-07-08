'use client'

import React, { useCallback, useEffect, useState, useRef } from 'react'
import { Card, Flex, Space, Tabs, TabsProps, Timeline } from 'antd'
import { useRouter } from 'next/router'
import toast from 'react-hot-toast'

import { CanvasCommentIcon, ResolveCommentIcon, PicturePlusIcon, Button } from '@gitmono/ui'

import Comment from '@/components/MrView/MRComment'
import { useGetIssueDetail } from '@/hooks/issues/useGetIssueDetail'
import { usePostIssueClose } from '@/hooks/issues/usePostIssueClose'
import { usePostIssueComment } from '@/hooks/issues/usePostIssueComment'
import { usePostIssueReopen } from '@/hooks/issues/usePostIssueReopen'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { SimpleNoteContent, SimpleNoteContentRef } from '@/components/SimpleNoteEditor/SimpleNoteContent';
import { EMPTY_HTML } from '@/atoms/markdown'
import { useHandleBottomScrollOffset } from '@/components/NoteEditor/useHandleBottomScrollOffset'
import { useUploadHelpers } from '@/hooks/useUploadHelpers';
import { ComposerReactionPicker } from '@/components/Reactions/ComposerReactionPicker';
import { trimHtml } from '@/utils/trimHtml'

interface IssueDetail {
  status: string
  conversations: Conversation[]
  title: string
}
interface Conversation {
  id: number
  conv_type: string
  comment: string
  created_at: number,
  updated_at: number,
  username: string,
}

interface detailRes {
  err_message: string
  data: IssueDetail
  req_result: boolean
}

export default function IssueDetailPage({ id }: { id: string }) {
  const [login, setLogin] = useState(false)
  const [info, setInfo] = useState<IssueDetail>({
    status: '',
    conversations: [],
    title: ''
  })
  const [buttonLoading, setButtonLoading] = useState<{ [key: string]: boolean }>({
    comment: false,
    close: false,
    reopen: false
  })
  const [isReactionPickerOpen, setIsReactionPickerOpen] = useState(false)
  const setLoading = (key: string, value: boolean) => {
    setButtonLoading((prev) => ({ ...prev, [key]: value }))
  }

  const { mutate: closeIssue } = usePostIssueClose()

  const { mutate: reopenIssue } = usePostIssueReopen()

  const { mutate: saveComment } = usePostIssueComment()

  const { data: issueDetailObj, error, isError, refetch } = useGetIssueDetail(id)

  const applyDetailData = (detail: detailRes | undefined) => {
    if (!detail || !detail.req_result) return
    setInfo({
      title: detail.data.title,
      status: detail.data.status,
      conversations: detail.data.conversations
    })
  }

  const fetchDetail = useCallback(() => {
    if (error || isError) return
    applyDetailData(issueDetailObj)
    setLogin(true)
  }, [issueDetailObj, error, isError])

  useEffect(() => {
    fetchDetail()
  }, [fetchDetail, id])

  const [_loadings, setLoadings] = useState<boolean[]>([])
  const router = useRouter()

  const set_to_loading = (index: number) => {
    setLoadings((prevLoadings) => {
      const newLoadings = [...prevLoadings]

      newLoadings[index] = true
      return newLoadings
    })
  }

  const cancel_loading = (index: number) => {
    setLoadings((prevLoadings) => {
      const newLoadings = [...prevLoadings]

      newLoadings[index] = false
      return newLoadings
    })
  }

  const close_issue = useCallback(() => {
    setLoading('close', true)
    set_to_loading(3)
    closeIssue(
      { link: id },
      {
        onSuccess: () => {
          router.push(`/${router.query.org}/issue`)
          cancel_loading(3)
        },
        onError: apiErrorToast,
        onSettled: () => setLoading('close', false)
      }
    )
  }, [id, router, closeIssue])

  const reopen_issue = useCallback(() => {
    setLoading('reopen', true)
    set_to_loading(3)
    reopenIssue(
      { link: id },
      {
        onSuccess: () => {
          router.push(`/${router.query.org}/issue`)
        },
        onError: apiErrorToast,
        onSettled: () => setLoading('reopen', false)
      }
    )
  }, [id, router, reopenIssue])

  const save_comment = useCallback(
    () => {
      const currentContentHTML = editorRef.current?.editor?.getHTML() ?? '<p></p>';
      
      if (trimHtml(currentContentHTML) === '') {
        toast.error('comment can not be empty!')
        return
      }
      setLoading('comment', true)
      set_to_loading(3)
      saveComment(
        { link: id, data: { content: currentContentHTML } },
        {
          onSuccess: async () => {
            editorRef.current?.clearAndBlur()
            const { data: issueDetailObj } = await refetch({ throwOnError: true })

            applyDetailData(issueDetailObj)
            cancel_loading(3)
            toast.success('comment successfully!')
          },
          onError: apiErrorToast,
          onSettled: () => setLoading('comment', false)
        }
      )
    },
    [id, refetch, saveComment]
  )

  const conv_items = info?.conversations.map((conv) => {
    let icon
    let children

    switch (conv.conv_type) {
      case 'Comment':
        icon = <CanvasCommentIcon />
        children = <Comment conv={conv} id={id} whoamI='issue' />
        break
      case 'Closed':
        icon = <ResolveCommentIcon />
        children = conv.comment
    }

    const element = {
      dot: icon,
      children: children
    }

    return element
  })

  const editorRef = useRef<SimpleNoteContentRef>(null);
  const onKeyDownScrollHandler = useHandleBottomScrollOffset({
    editor: editorRef.current?.editor
  })
  const { dropzone } = useUploadHelpers({
    upload: editorRef.current?.uploadAndAppendAttachments
  })

  const tab_items: TabsProps['items'] = [
    {
      key: '1',
      label: 'Conversation',
      children: (
        <Space direction='vertical' style={{ width: '100%' }}>
          <Timeline items={conv_items} />
          {info && info.status === 'open' && (
            <>
              <div className="prose flex flex-col w-full">
                <h2>Add a comment</h2>
                <input {...dropzone.getInputProps()} />
                <div className='border p-6 rounded-lg'>
                  <SimpleNoteContent
                    commentId="temp" //  Temporary filling, replacement later
                    ref={editorRef}
                    editable="all"
                    content={EMPTY_HTML}
                    autofocus={true}
                    onKeyDown={onKeyDownScrollHandler}
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
              </div>
              <Flex gap='small' justify={'flex-end'}>
                <Button
                  // loading={motivation === 'close' ? loadings[3] : undefined}
                  loading={buttonLoading.close}
                  disabled={!login}
                  onClick={() => close_issue()}
                >
                  Close issue
                </Button>
                <Button
                  loading={buttonLoading.comment}
                  disabled={!login}
                  onClick={() => save_comment()}
                >
                  Comment
                </Button>
              </Flex>
            </>
          )}
          {info && info.status === 'closed' && (
            <Flex gap='small' justify={'flex-end'}>
              <Button loading={buttonLoading.reopen} disabled={!login} onClick={() => reopen_issue()}>
                Reopen issue
              </Button>
            </Flex>
          )}
        </Space>
      )
    }
  ]

  return (
    <Card className='max-h-64 overflow-y-auto' title={info.title + ' #' + id}>
      <Tabs defaultActiveKey='1' items={tab_items} />
    </Card>
  )
}
