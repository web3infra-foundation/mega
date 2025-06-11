'use client'

import React, { useEffect, useMemo, useState } from 'react'
import { Theme } from '@radix-ui/themes'
import { Flex, Layout } from 'antd'
import { useParams } from 'next/navigation'

import { CommonResultVecTreeCommitItem } from '@gitmono/types/generated'
import { LoadingSpinner } from '@gitmono/ui'

import CodeTable from '@/components/CodeView/CodeTable'
import BreadCrumb from '@/components/CodeView/TreeView/BreadCrumb'
import CloneTabs from '@/components/CodeView/TreeView/CloneTabs'
import RepoTree from '@/components/CodeView/TreeView/RepoTree'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetTreeCommitInfo } from '@/hooks/useGetTreeCommitInfo'
import { useGetTreePathCanClone } from '@/hooks/useGetTreePathCanClone'

function TreeDetailPage() {
  const params = useParams()
  const { path = [] } = params as { path?: string[] }
  const new_path = '/' + path?.join('/')

  const { data: TreeCommitInfo } = useGetTreeCommitInfo(new_path)

  type DirectoryType = NonNullable<CommonResultVecTreeCommitItem['data']>
  const directory: DirectoryType = useMemo(() => TreeCommitInfo?.data ?? [], [TreeCommitInfo])

  const { data: canClone } = useGetTreePathCanClone({ path: new_path })
  const [readmeContent, setReadmeContent] = useState('')
  const [endpoint, setEndPoint] = useState('')

  useEffect(() => {
    const fetchData = async () => {
      try {
        const readmeContent = await getReadmeContent(new_path, directory)

        setReadmeContent(readmeContent)
        const endpoint = await getEndpoint()

        setEndPoint(endpoint)
      } catch (error) {
        // eslint-disable-next-line no-console
        console.error('Error fetching data:', error)
      }
    }

    fetchData()
  }, [new_path, directory])

  const treeStyle = {
    borderRadius: 8,
    overflow: 'hidden',
    width: 'calc(20% - 8px)',
    maxWidth: 'calc(20% - 8px)',
    background: '#fff'
  }

  const codeStyle = {
    borderRadius: 8,
    overflow: 'hidden',
    width: 'calc(80% - 8px)',
    background: '#fff'
  }

  const breadStyle = {
    minHeight: 30,
    borderRadius: 8,
    overflow: 'hidden',
    width: 'calc(100% - 8px)',
    background: '#fff'
  }

  return (
    <div className='relative m-2 h-screen overflow-hidden'>
      {!TreeCommitInfo ? (
        <div className='align-center container absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 justify-center'>
          <LoadingSpinner />
        </div>
      ) : (
        <Flex gap='middle' wrap>
          <Layout style={breadStyle}>
            <BreadCrumb path={path} />
            {canClone?.data && (
              <Flex justify={'flex-end'}>
                <CloneTabs endpoint={endpoint} />
              </Flex>
            )}
          </Layout>
          {/* tree */}
          <Layout style={treeStyle}>
            <RepoTree directory={directory} />
          </Layout>
          <Layout style={codeStyle}>
            <CodeTable directory={directory} loading={!TreeCommitInfo} readmeContent={readmeContent} />
          </Layout>
        </Flex>
      )}
    </div>
  )
}

async function getReadmeContent(pathname: string, directory: any) {
  let readmeContent = ''

  for (const project of directory || []) {
    if (project.name === 'README.md' && project.content_type === 'file') {
      const res = await fetch(`/api/blob?path=${pathname}/README.md`)
      const response = await res.json()

      readmeContent = response.data.data
      break
    }
  }
  return readmeContent
}

async function getEndpoint() {
  const res = await fetch(`/host`)
  const response = await res.json()

  return response.endpoint
}

TreeDetailPage.getProviders = (
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
  pageProps: React.JSX.IntrinsicAttributes & { children?: React.ReactNode | undefined }
) => {
  return (
    <AuthAppProviders {...pageProps}>
      <Theme>
        <AppLayout {...pageProps}>{page}</AppLayout>
      </Theme>
    </AuthAppProviders>
  )
}

export default TreeDetailPage
