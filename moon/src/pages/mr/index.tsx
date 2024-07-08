import { useRouter } from 'next/router'
import MergeList from "../../components/MergeList";

export default function MergeRequestPage({ mrList }) {
    const router = useRouter();
    return (
        <div>
            <MergeList mrList={mrList} />
        </div>
    )
}

export async function getServerSideProps(context) {
    const res = await fetch(`http://localhost:3000/api/mr`);
    const response = await res.json();
    const mrList = response.data;
    return {
        props: {
            mrList,
        },
    };
}