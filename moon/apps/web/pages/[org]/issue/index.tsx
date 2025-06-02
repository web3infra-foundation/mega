import IssuePage from '@/components/Issues/IssuePage'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

const OrganizationIssuePage: PageWithLayout<any> = () => {
  return (
    <>
      <IssuePage />
    </>
  )
}

OrganizationIssuePage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrganizationIssuePage
