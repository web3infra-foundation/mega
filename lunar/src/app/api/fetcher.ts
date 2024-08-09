import useSWR from "swr";

const endpoint = process.env.NEXT_PUBLIC_API_URL;
const relay = process.env.NEXT_PUBLIC_RELAY_API_URL;

export class FetchError extends Error {
  info: any;
  status: number;

  constructor(message: string, info: any, status: number) {
    super(message);
    this.info = info;
    this.status = status;
  }
}

const fetcher = async url => {
  const res = await fetch(url)
  if (!res.ok) {
    const error = new Error('An error occurred while fetching the data.')
    const errorInfo = await res.json();
    throw new FetchError('An error occurred while fetching the data.', errorInfo, res.status);
  }
  return res.json()
}


export function useTreeCommitInfo(path) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/tree/commit-info?path=${path}`, fetcher, {
    dedupingInterval: 60000,
  })
  return {
    tree: data,
    isTreeLoading: isLoading,
    isTreeError: error,
  }
}

export function useBlobContent(path) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/blob?path=${path}`, fetcher, {
    dedupingInterval: 60000,
  })
  return {
    blob: data,
    isBlobLoading: isLoading,
    isBlobError: error,
  }
}

export function useRepoList() {
  const { data, error, isLoading } = useSWR(`${relay}/relay/api/v1/repo_list`, fetcher, {
    dedupingInterval: 30000,
  })
  return {
    data: data,
    isLoading,
    isError: error,
  }
}
