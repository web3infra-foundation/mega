import { SCOPE_COOKIE_NAME } from '@gitmono/config';
import { ApiErrorTypes, OrganizationsPostRequest } from '@gitmono/types';
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

    const organizations = await apiClient.organizationMemberships
      .getOrganizationMemberships()
      .request({ headers })
      .then((res) => res.map((m) => m.organization))

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
      const defaultOrgData: OrganizationsPostRequest = {
        name: "My First Organization",
        slug: "my-first-organization",
        avatar_path: null,
        role: "software-engineer",
        org_size: "1",
        source: "google",
        why: "Please enter your purpose."
      }

      apiClient.organizations
        .postOrganizations()
        .request(defaultOrgData)
        .catch((e) => {
          throw new Error("postOrganizationError: " + e.message)
        })

      //新建organization时会自动创建第一个channel(General)

      // const defaultChannel = "default channel"
      //
      // apiClient.organizations
      //   .postProjects()
      //   .request(`my-first-organization`,{ name: defaultChannel }, {headers})
      //   .catch(e => {
      //       throw new Error("postProjectError: " + e.message)
      //     }
      //   )

      if (device.type === 'mobile') {
        return {
          redirect: {
            destination: `/${defaultOrgData.slug}/${query.path ?? 'home'}`,
            permanent: false
          }
        }
      }
      return {
        redirect: {
          destination: `/${defaultOrgData.slug}/${query.path ?? ''}`,
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
