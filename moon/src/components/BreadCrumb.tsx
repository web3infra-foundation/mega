import 'github-markdown-css/github-markdown-light.css'
import { useRouter } from 'next/navigation'
import { Breadcrumb } from 'antd/lib'
import styles from './BreadCrumb.module.css'

const Bread = ({ path }) => {
    const router = useRouter();
    const handleBreadcrumbClick = async (index) => {
        router.push(`/tree/${path.slice(0, index + 1).join('/')}`);
    };

    const breadCrumbItems = path.map((path, index) => ({
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
