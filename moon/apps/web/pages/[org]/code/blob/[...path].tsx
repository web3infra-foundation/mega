import React from 'react'
import { Flex, Layout } from 'antd'
import Bread from '@/components/CodeView/TreeView/BreadCrumb'
import CodeContent from '@/components/CodeView/BlobView/CodeContent'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetBlob } from '@/hooks/useGetBlob'
import { useRouter } from 'next/router'
import { CommentSection } from '@/components/CodeView/BlobView/CommentSection'

const codeStyle = {
  borderRadius: 8,
  width: 'calc(85% - 8px)',
  background: '#fff'
}

const breadStyle = {
  minHeight: 30,
  borderRadius: 8,
  overflow: 'hidden',
  width: 'calc(100% - 8px)',
  background: '#fff'
}

interface Comment {
  id: string
  content: string
  author: {
    id: string
    name: string
    avatar?: string
  }
  createdAt: Date
  replies?: Comment[]
}

function BlobPage() {
  const { path = [] } = useRouter().query as { path?: string[] }
  const new_path = '/' + path.join('/')
  const fileContent = useGetBlob({ path: new_path }).data?.data?? ""
  const mockComments: Comment[] = [
    {
      id: '1',
      content: '这段代码逻辑很清晰，建议可以添加一些错误处理。',
      author: {
        id: '1',
        name: '张三',
        avatar: ''
      },
      createdAt: new Date('2024-12-01 10:30:00'),
      replies: []
    },
    {
      id: '2',
      content: '同意。',
      author: {
        id: '2',
        name: '李四',
        avatar: ''
      },
      createdAt: new Date('2024-12-01 10:30:00'),
      replies: [
        {
          id: '3',
          content: '好的，收到。',
          author: {
            id: '3',
            name: '王五',
            avatar: ''
          },
          createdAt: new Date('2024-12-01 10:30:00')
        }
      ]
    }
  ]
  const handleAddComment = (__content: string, __lineNumber?: number) => {
    //wait for complete
  }

  const handleReplyComment = (__commentId: string, __content: string) => {
    //wait for complete
  }


  return (
    <div style={{overflow: 'auto'}}>
      <Flex gap='middle' wrap>
        <Layout style={breadStyle}>
          <Bread path={path} />
        </Layout>
        <Layout style={codeStyle}>
          <CodeContent fileContent={fileContent} />
        </Layout>
        <Layout>
          {/* @ts-ignore */}
          <CommentSection comments={mockComments} onAddComment={handleAddComment} onReplyComment={handleReplyComment} />
        </Layout>
      </Flex>
    </div>
  )
}

BlobPage.getProviders = (
  page:
    | string
    | number
    | boolean
    | React.ReactElement<any, string | React.JSXElementConstructor<any>>
    | Iterable<React.ReactNode>
    | React.ReactPortal
    | Promise<React.AwaitedReactNode>
    | null
    | undefined,
  pageProps: React.JSX.IntrinsicAttributes & { children?: React.ReactNode }
) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default BlobPage