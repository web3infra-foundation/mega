'use client'

import DataList from '@/components/DataList'
import { useRepoList } from '../api/fetcher';
import { Skeleton } from "antd";


export default function RepoPage() {
  const { data, isLoading, isError } = useRepoList();

  return (
    <div>
      {
        isLoading &&
        <Skeleton />
      }
      {
        !isLoading &&
        <DataList data={data} />
      }
    </div>
  )
}