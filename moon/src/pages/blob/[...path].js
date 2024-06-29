import { useRouter } from 'next/router'
import axios from 'axios';
import CodeContent from '@/components/CodeContent';

export default function Page({ fileContent }) {
    const router = useRouter();
    return (
        <div>
            <CodeContent fileContent={fileContent} />
        </div>
    )
}

export async function getServerSideProps(context) {
    const MEGA_URL = 'http://localhost:8000';
    const { path } = context.query;
    var fileContent = '';
    try {
        const fileResponse = await axios.get(`${MEGA_URL}/api/v1/blob?path=/${path.join('/')}`, { withCredentials: true });
        fileContent = fileResponse.data.data;
    } catch (error) {
        console.error("Error fetching file content:", error);
    }
    return {
        props: {
            fileContent,
        },
    };
}