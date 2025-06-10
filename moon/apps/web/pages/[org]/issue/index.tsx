// import IssuePage from '@/components/Issues/IssuePage'
import { IssueIndex } from '@/components/Issues/IssueIndex'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { ScopeProvider } from '@/contexts/scope'
import { PageWithLayout } from '@/utils/types'

const OrganizationIssuePage: PageWithLayout<any> = () => {
  return (
    <>
      <IssueIndex />
    </>
  )
}

OrganizationIssuePage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <ScopeProvider>
        <AppLayout {...pageProps}>{page}</AppLayout>
      </ScopeProvider>
    </AuthAppProviders>
  )
}

export default OrganizationIssuePage
