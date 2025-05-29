import React from 'react'
import { Flex, Layout } from 'antd'
import Bread from '@/components/CodeView/TreeView/BreadCrumb'
import CodeContent from '@/components/CodeView/BlobView/CodeContent'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetBlob } from '@/hooks/useGetBlob'
import { useRouter } from 'next/router'

const treeStyle = {
  borderRadius: 8,
  overflow: 'hidden',
  width: 'calc(15% - 8px)',
  maxWidth: 'calc(15% - 8px)',
  background: '#fff'
}

const codeStyle = {
  borderRadius: 8,
  overflow: 'hidden',
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

function BlobPage() {
  const { path = [] } = useRouter().query as { path?: string[] }
  const new_path = '/' + path.join('/')
  const fileContent = useGetBlob({ path: new_path }).data?.data?? ""


  return (
    <div>
      <Flex gap='middle' wrap>
        <Layout style={breadStyle}>
          <Bread path={path} />
        </Layout>
        <Layout style={treeStyle}>{/* <RepoTree directory={directory} /> */}</Layout>
        <Layout style={codeStyle}>
          <CodeContent fileContent={fileContent} />
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