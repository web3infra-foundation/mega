import MergeDetail from "../../components/MergeDetail";


export default function MRDetailPage({ mrDetail }) {
    return (
        <div>
            <MergeDetail mrDetail={mrDetail}/>
        </div>
    )
}

export async function getServerSideProps(context) {
    const { id } = context.query;
    const res = await fetch(`http://localhost:3000/api/mr?id=${id}`);
    const response = await res.json();
    const mrDetail = response.data;
    return {
        props: {
            mrDetail,
        },
    };
}