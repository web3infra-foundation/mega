import Head from 'next/head'

import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'
import TestView from '@/components/TestView'

const OrganizationTestPage: PageWithLayout<any> = () => {

  return (
    <>
      <Head>
        <title>Test</title>
      </Head>

      <TestView />
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
