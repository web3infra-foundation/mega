import { useRouter } from 'next/router'
import CodeContent from '../../components/CodeContent';
import Bread from '../../components/BreadCrumb';

export default function BlobPage({ fileContent }) {
    const router = useRouter();
    return (
        <div>
            <Bread />
            <CodeContent fileContent={fileContent} />
        </div>
    )
}

export async function getServerSideProps(context) {
    const { path } = context.query;
    const res = await fetch(`http://localhost:3000/api/blob?path=/${path.join('/')}`);
    const response = await res.json();
    var fileContent = response.data;
    return {
        props: {
            fileContent,
        },
    };
}