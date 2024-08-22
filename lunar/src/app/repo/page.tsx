'use client'

import RepoList from '@/components/RepoList'
import { useMegaStatus, useRepoList } from '@/app/api/fetcher';
import { Skeleton } from "antd";


export default function RepoPage() {
  const { repo, isRepoLoading, isRepoError } = useRepoList();

  if (isRepoLoading) return <Skeleton />;
  return (
    <RepoList data={repo} />
  )
}