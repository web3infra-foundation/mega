import React from 'react';
import 'github-markdown-css/github-markdown-light.css'
import { useRouter } from 'next/router';
import { BreadcrumbLabel } from '@/components/Titlebar/BreadcrumbTitlebar'
import { Link } from '@gitmono/ui'
import { UrlObject } from 'url';

const Breadcrumb = ({ path }:any) => {
  const router = useRouter();
  const scope = router.query.org as string
  
  const breadCrumbItems = path?.map((subPath: any, index: number) => {
    const href = `/${scope}/code/tree/${path.slice(0, index + 1).join('/')}`;

    return {
      title: subPath,
      href: href,
      isLast: index === path.length - 1,
    };
  });

    return (
      <div className='flex items-center overflow-x-auto p-2 no-scrollbar mt-2'>

        {breadCrumbItems?.map((item: { isLast: any; title: string; href: string | UrlObject; }, index: number) => (
        <React.Fragment key={item.title}>
          {/* displayed after the home item and before non-last items */}
          {index > 0 && (
            <span className="text-gray-400">/</span>
          )}
          {/* Current breadcrumb item */}
          {item.isLast ? (
            // last item
            <BreadcrumbLabel>
              {item?.title}
            </BreadcrumbLabel>
          ) : (
            // middle item
            <Link href={item?.href} >
              <BreadcrumbLabel className="ml-1">{item?.title}</BreadcrumbLabel>
            </Link>
          )}
        </React.Fragment>
))}
      </div>
    );
};

export default Breadcrumb;
