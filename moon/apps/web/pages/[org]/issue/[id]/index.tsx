import { GetServerSideProps } from 'next'

import IssueDetailPage from '@/components/Issues/IssueDetailPage'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

export const getServerSideProps: GetServerSideProps = async ({ query }) => {
  if (!query.id) {
    return {
      redirect: {
        destination: `/${query.org}/issue`,
        permanent: false
      }
    }
  }
  return {
    props: {
      id: query.id
    }
  }
}

const OrganizationIssueDetailPage: PageWithLayout<any> = ({ id }) => {
  return (
    <>
      <IssueDetailPage id={id} />
    </>
  )
}

OrganizationIssueDetailPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrganizationIssueDetailPage
