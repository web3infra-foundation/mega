'use client'

import React, { useCallback, useEffect, useState } from 'react'
// import { CloseCircleOutlined, CommentOutlined } from '@ant-design/icons'
import { Button, Card, Flex, Space, Tabs, TabsProps, Timeline } from 'antd'
import { useRouter } from 'next/router'
import toast from 'react-hot-toast'

import Comment from '@/components/MrView/MRComment'
import RichEditor from '@/components/MrView/rich-editor/RichEditor'
import { useGetIssueDetail } from '@/hooks/issues/useGetIssueDetail'
import { usePostIssueClose } from '@/hooks/issues/usePostIssueClose'
import { usePostIssueComment } from '@/hooks/issues/usePostIssueComment'
import { usePostIssueReopen } from '@/hooks/issues/usePostIssueReopen'
import { apiErrorToast } from '@/utils/apiErrorToast'

interface IssueDetail {
  status: string
  conversations: Conversation[]
  title: string
}
interface Conversation {
  id: number
  user_id: number
  conv_type: string
  comment: string
  created_at: number
}

interface detailRes {
  err_message: string
  data: IssueDetail
  req_result: boolean
}

export default function IssueDetailPage({ id }: { id: string }) {
  const [editorState, setEditorState] = useState('')
  const [login, setLogin] = useState(false)
  const [info, setInfo] = useState<IssueDetail>({
    status: '',
    conversations: [],
    title: ''
  })
  const [editorHasText, setEditorHasText] = useState(false);
  const [buttonLoading, setButtonLoading] = useState<{ [key: string]: boolean }>({
    comment: false,
    close: false,
    reopen: false
  })

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
  }, [fetchDetail])

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
    (comment: string) => {
      if (JSON.parse(comment).root.children[0].children.length === 0) {
        toast.error('comment can not be empty!')
        return
      }
      setLoading('comment', true)
      set_to_loading(3)
      saveComment(
        { link: id, data: { content: comment } },
        {
          onSuccess: async () => {
            setEditorState('')
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
        // icon = <CommentOutlined />
        children = <Comment conv={conv} id={id} whoamI='issue' />
        break
      case 'Closed':
        // icon = <CloseCircleOutlined />
        children = conv.comment
    }

    const element = {
      dot: icon,
      children: children
    }

    return element
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
              <h1>Add a comment</h1>
              <RichEditor setEditorState={setEditorState} setEditorHasText={setEditorHasText}/>
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
                  disabled={editorState === '' || !login || !editorHasText}
                  onClick={() => save_comment(editorState)}
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
