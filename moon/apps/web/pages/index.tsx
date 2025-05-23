import { SCOPE_COOKIE_NAME } from '@gitmono/config';
import { ApiErrorTypes } from '@gitmono/types';
import { GetServerSideProps } from 'next';
import { userAgentFromString } from 'next/server';
import { apiCookieHeaders } from '@/utils/apiCookieHeaders';
import { apiClient, signinUrl } from '@/utils/queryClient';

export default function IndexPage() {
  return <></>
}

export const getServerSideProps: GetServerSideProps = async ({ req, query }) => {
  try {
    const { device } = userAgentFromString(req.headers['user-agent'])
    const headers = apiCookieHeaders(req.cookies)
    //todo: Encrypted storage or server-side retrieval
    const admin_headers = {
      Cookie: '_campsite_api_session=sHKWgLii9No0CuFrr4q29G3iGisYILKp1776kwY%2Fb5xK24RFioyvcA9yPZDlK9ZavemJLQtjvSpuwwYE8LTyc6KF0wtaJFFYBqhx9YVRBfOXSFSRQeTi8A2X2GXe6SrEvYSTWDdPVI0et2mpvt2RwR6ajbolpRw57D9XBdLrGaQnIZ6YnUbXOmbPSDY34lyH4X%2FhX6oS1ms%2FPMtqbZh%2BAZiaXE0l2pjl9iWsvGHN1YpUv8kwcD5KcqtvBpJwaSChG7lXDkVy4SP2k9PzKmL8Zui79sDGZEJ5D8oRKo6a9uVy%2FeROaD1ewlHXylU%2FdWHcR%2Ft6Z1TrjxPWIDs6VB5nDudxFgap5XNjOvH4%2FG9t7pi%2FGfvteu%2B1yJ8%2Fkiz1jlg%2B7onVGNKJT2mmsHaJ6JF1QMAj0UgakG0a7hc8ui77v0VxE5avGc9I0i1z8mo4hX0k8yOVOPZkp5GwQHsBJK0%2BKmg%2Fsw%2BG5Fc%3D--qs0r30iu9tg2D6wn--EZ%2FzQJVCBVvWjVKG13PnLw%3D%3D',
      'x-campsite-ssr-secret': ''
    }

    const organizations = await apiClient.organizationMemberships
      .getOrganizationMemberships()
      .request({ headers })
      .then((res) =>
        res.map(m => m.organization)
          .filter(o => o !== null)
      )

    // if we have orgs redirect to one of the user orgs,
    // otherwise redirect to the new org page
    if (organizations.length > 0) {
      let org = organizations[organizations.length - 1]
      const scope = req.cookies[SCOPE_COOKIE_NAME]

      if (scope) {
        const scopeOrg = organizations.find((o) => o?.slug == scope)

        if (scopeOrg) {
          org = scopeOrg
        }
      }

      if (device.type === 'mobile') {
        return {
          redirect: {
            destination: `/${org?.slug}/${query.path ?? 'home'}`,
            permanent: false
          }
        }
      }
      return {
        redirect: {
          destination: `/${org?.slug}/${query.path ?? ''}`,
          permanent: false
        }
      }
    } else {
      const userData = await apiClient.users
        .getMe()
        .request({ headers })
        .then((res) => res)

      await apiClient.organizations.postInvitations().request(
        'mega',
        {
          invitations: [
            {
              email: userData.email,
              role: 'member'
            }
          ]
        },
        { headers: admin_headers }
      )

      const invitations = await apiClient.users.getMeOrganizationInvitations().request({ headers })

      for (let i of invitations) {
        if (i.organization?.slug === 'mega') {
          await apiClient.invitationsByToken.postInvitationsByTokenAccept().request(i.token!!, { headers })
        }
      }

      if (device.type === 'mobile') {
        return {
          redirect: {
            destination: `/mega/${query.path ?? 'home'}`,
            permanent: false
          }
        }
      }
      return {
        redirect: {
          destination: `/mega/${query.path ?? ''}`,
          permanent: false
        }
      }
    }
  } catch (e: any) {
    if (e.name === ApiErrorTypes.AuthenticationError) {
      return {
        redirect: {
          permanent: false,
          destination: signinUrl({ from: req?.url })
        }
      }
    } else if (e.name === ApiErrorTypes.NotFoundError) {
      return { notFound: true }
    } else {
      throw e
    }
  }
}
