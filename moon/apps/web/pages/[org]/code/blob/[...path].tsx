import React from 'react'
import { useRouter } from 'next/router'

import CodeContent from '@/components/CodeView/BlobView/CodeContent'
import CommitHistory, { CommitInfo } from '@/components/CodeView/CommitHistory'
import BreadCrumb from '@/components/CodeView/TreeView/BreadCrumb'
import RepoTree from '@/components/CodeView/TreeView/RepoTree'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetBlob } from '@/hooks/useGetBlob'
import { Theme } from '@radix-ui/themes'

const codeStyle = {
  borderRadius: 8,
  background: '#fff',
  border: '1px solid #d1d9e0',
  margin: '0 8px',
  flexGrow: 1,
  minWidth: 0
}

const treeStyle = {
  borderRadius: 8,
  overflow: 'hidden',
  width: '400px',
  background: '#fff'
}

function BlobPage() {
  const { path = [] } = useRouter().query as { path?: string[] }
  const new_path = '/' + path.join('/')
  const fileContent = useGetBlob({ path: new_path }).data?.data ?? ''
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
    <Theme>
      <div className='relative m-4 h-screen overflow-auto'>
          <BreadCrumb path={path} />
          {/* tree */}
          <div className='flex gap-4'>
            <div style={treeStyle}>
              <RepoTree />
            </div>

            <div style={codeStyle}>
              <div className='m-2'>
                <CommitHistory flag={'details'} info={commitInfo} />
              </div>
              <div className='flex w-full flex-wrap gap-4'>
                <div style={codeStyle}>
                  <CodeContent fileContent={fileContent} path={path} />
                </div>
              </div>
            </div>
          </div>
      </div>
    </Theme>
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