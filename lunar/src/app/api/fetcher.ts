import useSWR, { Fetcher } from "swr";
import { invoke } from '@tauri-apps/api/core';

const endpoint = process.env.NEXT_PUBLIC_API_URL;
const relay = process.env.NEXT_PUBLIC_RELAY_API_URL;

export interface ApiResult<T> {
  req_result: boolean,
  data: T,
  err_message: string
}

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
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/mono/tree/commit-info?path=${path}`, fetcher, {
    dedupingInterval: 30000,
  })
  return {
    tree: data,
    isTreeLoading: isLoading,
    isTreeError: error,
  }
}

export function useBlobContent(path) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/mono/blob?path=${path}`, fetcher, {
    dedupingInterval: 60000,
  })
  return {
    blob: data,
    isBlobLoading: isLoading,
    isBlobError: error,
  }
}

export function useMRList(status) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/mono/mr/list?status=${status}`, fetcher, {
    dedupingInterval: 60000,
  })
  return {
    mrList: data,
    isMRLoading: isLoading,
    isMRError: error,
  }
}

export function useMRDetail(id) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/mono/mr/${id}/detail`, fetcher, {
    dedupingInterval: 60000,
  })
  return {
    mrDetail: data,
    isMRLoading: isLoading,
    isMRError: error,
  }
}

export function useMRFiles(id) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/mono/mr/${id}/files`, fetcher, {
    dedupingInterval: 60000,
  })
  return {
    mrFiles: data,
    isMRLoading: isLoading,
    isMRError: error,
  }
}

export function useRepoList() {
  const { data, error, isLoading } = useSWR(`${relay}/relay/api/v1/repo_list`, fetcher, {
    dedupingInterval: 30000,
  })
  return {
    repo: data,
    isRepoLoading: isLoading,
    isRepoError: error,
  }
}

export function usePeerId() {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/mega/ztm/peer_id`, fetcher, {
    dedupingInterval: 60000,
  })
  return {
    peerId: data,
    isLoading: isLoading,
    isError: error,
  }
}

export function useRepoFork(identifier) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/mega/ztm/repo_fork?identifier=${identifier}`, {
    dedupingInterval: 60000,
  })
  return {
    url: data,
    isForkLoading: isLoading,
    isForkError: error,
  }
}

// export function usePublishRepo(path: string) {
//   const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/mega/ztm/repo_provide?path=${path}`, fetcher)
//   return {
//     data: data,
//     isLoading,
//     isError: error,
//   }
// }

export const tauriFetcher: Fetcher<any, [string, { [key: string]: any }]> = ([key, args]) => {
  return invoke(key, args);
};

export function useMegaStatus() {
  const { data, error, isLoading } = useSWR(
    ['mega_service_status', {}],
    tauriFetcher
  );

  return {
    status: data,
    isLoading,
    isError: error,
  };
}

// normal fetch 
export async function requestPublishRepo(data) {
  const response = await fetch(`${endpoint}/api/v1/mega/ztm/repo_provide`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    const errorResponse = await response.text();
    const errorMessage = errorResponse || 'Failed to publish repo';
    throw new Error(errorMessage);
  }
  return response.json();
}
