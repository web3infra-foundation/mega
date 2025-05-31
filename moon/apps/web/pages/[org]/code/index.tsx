import { Theme } from '@radix-ui/themes'
import Head from 'next/head'

import CodeView from '@/components/CodeView'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

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
      <Theme>
        <AppLayout {...pageProps}>{page}</AppLayout>
      </Theme>
    </AuthAppProviders>
  )
}

export default OrganizationTestPage
