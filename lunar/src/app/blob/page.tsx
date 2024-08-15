'use client'
import CodeContent from '@/components/CodeContent';
import Bread from '@/components/BreadCrumb';
import { useSearchParams } from 'next/navigation';
import { useBlobContent } from '../api/fetcher';
import { Skeleton } from "antd/lib";


export default function BlobPage() {
    const searchParams = useSearchParams();
    const path = searchParams.get('path');

    const { blob, isBlobLoading, isBlobError } = useBlobContent(`${path}`);
    if (isBlobLoading) return <Skeleton />;

    return (
        <div>
            <Bread path={path} />
            <CodeContent fileContent={blob.data} />
        </div>
    )
}
