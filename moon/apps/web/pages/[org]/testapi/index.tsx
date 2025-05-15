import Head from 'next/head'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useTestInfo } from '@/hooks/useTestInfo'
import { PageWithLayout } from '@/utils/types'

const TestApiPage: PageWithLayout<any> = () => {
  const { data, isLoading, error } = useTestInfo('/third-part')

  return (
    <>
      <Head>
        <title>API Test Page</title>
      </Head>
      <div className="p-4">
        <h1 className="text-2xl font-bold mb-4">API Test Results</h1>
        {isLoading && <div>Loading...</div>}
        {error && <div className="text-red-500">Error: {error.message}</div>}
        {data && (
          <div className="bg-gray-100 p-4 rounded">
            <pre>{JSON.stringify(data, null, 2)}</pre>
          </div>
        )}
      </div>
    </>
  )
}

TestApiPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default TestApiPage
