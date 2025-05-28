import Head from 'next/head'

import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'
import CodeView from '@/components/CodeView'

const OrganizationTestPage: PageWithLayout<any> = () => {

  return (
    <>
      <Head>
        <title>Code</title>
      </Head>

      <CodeView />
    </>
  )
}

OrganizationTestPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrganizationTestPage
