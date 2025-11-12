import Head from 'next/head'

import CLView from '@/components/ClView'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

const OrganizationCLPage: PageWithLayout<any> = () => {
  return (
    <>
      <Head>
        <title>CL</title>
      </Head>
      <CLView />
    </>
  )
}

OrganizationCLPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrganizationCLPage
