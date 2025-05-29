import 'github-markdown-css/github-markdown-light.css'
import { Breadcrumb } from 'antd'
import { useRouter } from 'next/router';

const Bread = ({ path }:any) => {
  const router = useRouter();
  const scope = router.query.org as string
  
  const breadCrumbItems = path?.map((sub_path: any, index: number) => {
      if (index == path?.length - 1) {
          return {
              title: sub_path,
          };
      } else {
          const href = `/${scope}/code/tree/${path?.slice(0, index + 1).join('/')}`;

          return {
              title: sub_path,
              href: href,
          };
      }
    });

    return (
      <div className='m-4'>
        <Breadcrumb items={breadCrumbItems}/>
      </div>
    );
};

export default Bread;
