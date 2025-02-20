'use client'
import MergeDetail from "@/components/MergeDetail";
import { useMRDetail } from "../api/fetcher";
import { Skeleton } from "antd/lib";
import { useSearchParams } from "next/navigation";
import { Suspense } from 'react'

export default function Page() {
    return (
        <Suspense>
            <MRDetailPage />
        </Suspense>
    );
}

function MRDetailPage() {
    const searchParams = useSearchParams();
    const id = searchParams.get('id');
    const { mrDetail, isMRLoading, isMRError } = useMRDetail(id);
    if (isMRLoading) return <Skeleton />;

    return (
        <div>
            <MergeDetail mrDetail={mrDetail.data} />
        </div>
    )
}
