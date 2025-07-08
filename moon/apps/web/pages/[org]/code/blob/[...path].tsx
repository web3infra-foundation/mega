import React from 'react'
import { Flex, Layout } from 'antd'
import BreadCrumb from '@/components/CodeView/TreeView/BreadCrumb'
import CodeContent from '@/components/CodeView/BlobView/CodeContent'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetBlob } from '@/hooks/useGetBlob'
import { useRouter } from 'next/router'
import CommitHistory, { CommitInfo } from '@/components/CodeView/CommitHistory'
import RepoTree from '@/components/CodeView/TreeView/RepoTree'

const codeStyle = {
  borderRadius: 8,
  background: '#fff',
  border: '1px solid #d1d9e0',
  margin: '0 8px',
  width: 'calc(80% - 8px)',
}

const treeStyle = {
  borderRadius: 8,
  overflow: 'hidden',
  width: 'calc(20% - 8px)',
  background: '#fff'
}

function BlobPage() {
  const { path = [] } = useRouter().query as { path?: string[] }
  const new_path = '/' + path.join('/')
  const fileContent = useGetBlob({ path: new_path }).data?.data?? ""
  const commitInfo: CommitInfo = {
    user: {
      avatar_url: 'https://avatars.githubusercontent.com/u/112836202?v=4&size=40',
      name: 'yetianxing2014'
    },
    message: 'feat: migrate campsite to mega',
    hash: '5fe4235',
    date: '3 months ago'
  }

  return (
    <div style={{overflow: 'auto'}}>
      <Flex vertical gap='middle'>
        <Layout>
          <BreadCrumb path={path} />
        </Layout>
        {/* tree */}
        <Flex>
        <Layout style={treeStyle}>
          <RepoTree  flag={'detail'} />
        </Layout>

        <Layout style={codeStyle}>
          <Layout className='m-2'>
            <CommitHistory flag={'details'} info={commitInfo}/>
          </Layout>
          <Flex gap='middle' wrap>
            <Layout style={codeStyle}>
              <CodeContent fileContent={fileContent} path={path} />
            </Layout>
          </Flex>
        </Layout>
        </Flex>
      </Flex>
    </div>
  )
}

BlobPage.getProviders = (
  page:
    | string
    | number
    | boolean
    | React.ReactElement
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
