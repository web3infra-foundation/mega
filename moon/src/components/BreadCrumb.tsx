
import 'github-markdown-css/github-markdown-light.css'
import { useRouter } from 'next/router'
import { Breadcrumb } from 'antd/lib'
import styles from './BreadCrumb.module.css'

const Bread = () => {
    const router = useRouter();
    const { path } = router.query;
    const safePath = Array.isArray(path) ? path : [];

    const handleBreadcrumbClick = async (index) => {
        router.push(`/tree/${safePath.slice(0, index + 1).join('/')}`);
    };

    const breadCrumbItems = safePath.map((path, index) => ({
        title: path,
        onClick: () => handleBreadcrumbClick(index),
    }));

    return (
        <Breadcrumb className={styles.breadCrumb}
            items={breadCrumbItems}
        />
    );
};

export default Bread;
