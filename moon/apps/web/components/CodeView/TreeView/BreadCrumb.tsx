import React from 'react'

import 'github-markdown-css/github-markdown-light.css'

import { UrlObject } from 'url'
import { useRouter } from 'next/router'

import { Link } from '@gitmono/ui'

import { BreadcrumbLabel } from '@/components/Titlebar/BreadcrumbTitlebar'

const Breadcrumb = ({ path }: any) => {
  const router = useRouter()
  const scope = router.query.org as string
  const refs = router.query.version as string

  const breadCrumbItems = path?.map((subPath: any, index: number) => {
    const pathPart = path.slice(0, index + 1).join('/')
    const href = `/${scope}/code/tree/${refs}/${pathPart}`

    return {
      title: subPath,
      href: href,
      isLast: index === path.length - 1
    }
  })

  return (
    <div className='no-scrollbar flex items-center gap-2 overflow-x-auto p-3'>
      {breadCrumbItems?.map((item: { isLast: any; title: string; href: string | UrlObject }, index: number) => (
        <React.Fragment key={item.title}>
          {/* displayed after the home item and before non-last items */}
          {index > 0 && <span className='text-tertiary'>/</span>}
          {/* Current breadcrumb item */}
          {item.isLast ? (
            // last item
            <BreadcrumbLabel>{item?.title}</BreadcrumbLabel>
          ) : (
            // middle item
            <Link href={item?.href}>
              <BreadcrumbLabel>{item?.title}</BreadcrumbLabel>
            </Link>
          )}
        </React.Fragment>
      ))}
    </div>
  )
}

export default Breadcrumb
