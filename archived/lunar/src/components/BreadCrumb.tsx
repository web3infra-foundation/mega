import 'github-markdown-css/github-markdown-light.css'
import { useRouter } from 'next/navigation'
import { Breadcrumb } from 'antd/lib'
import styles from './BreadCrumb.module.css'

const Bread = ({ path }) => {
    const router = useRouter();
    let path_arr = path.split('/').filter(Boolean);

    const breadCrumbItems = path_arr.map((path, index) => {
        if (index == path_arr.length - 1) {
            return {
                title: path,
            };
        } else {
            const href = '/tree?path=/' + path_arr.slice(0, index + 1).join('/');
            return {
                title: path,
                href: href,
            };
        }

    });

    return (
        <Breadcrumb className={styles.breadCrumb}
            items={breadCrumbItems}
        />
    );
};

export default Bread;
