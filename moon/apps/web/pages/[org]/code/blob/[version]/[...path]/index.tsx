import React from 'react'
import { Theme } from '@radix-ui/themes'
import { useParams } from 'next/navigation'

import CodeContent from '@/components/CodeView/BlobView/CodeContent'
import CommitHistory, { CommitInfo } from '@/components/CodeView/CommitHistory'
import BreadCrumb from '@/components/CodeView/TreeView/BreadCrumb'
import RepoTree from '@/components/CodeView/TreeView/RepoTree'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetBlob } from '@/hooks/useGetBlob'

const codeStyle = {
  borderRadius: 8,
  width: '80%',
  background: '#fff',
  height: 'calc(100vh - 96px)',
  overflow: 'auto',
  paddingRight: '8px'
}

const treeStyle = {
  borderRadius: 8,
  width: '20%',
  minWidth: '300px',
  flexShrink: 0,
  background: '#fff',
  height: 'calc(100vh - 96px)',
  overflow: 'auto',
  paddingRight: '8px'
}

function BlobPage() {
  const params = useParams()
  const { version, path = [] } = params as { version: string; path?: string[] }

  const refs = version === 'main' ? undefined : version
  const new_path = '/' + path.join('/')

  const { data: blobData, isLoading: isCodeLoading } = useGetBlob({ path: new_path, refs })
  const fileContent = blobData?.data ?? ''
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
      <div className='relative m-4 h-screen'>
        <BreadCrumb path={path} />
        {/* tree */}
        <div className='flex gap-4'>
          <div style={treeStyle}>
            <RepoTree />
          </div>

          <div style={codeStyle}>
            <div>
              <CommitHistory flag={'details'} info={commitInfo} />
            </div>
            <CodeContent fileContent={fileContent} path={path} isCodeLoading={isCodeLoading} />
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
