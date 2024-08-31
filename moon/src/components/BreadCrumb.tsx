import 'github-markdown-css/github-markdown-light.css'
import { useRouter } from 'next/navigation'
import { Breadcrumb } from 'antd/lib'
import styles from './BreadCrumb.module.css'

const Bread = ({ path }) => {
    const router = useRouter();
    const breadCrumbItems = path.map((sub_path, index) => {
        if (index == path.length - 1) {
            return {
                title: sub_path,
            };
        } else {
            const href = '/tree/' + path.slice(0, index + 1).join('/');
            return {
                title: sub_path,
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
