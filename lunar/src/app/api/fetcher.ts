import useSWR from "swr";

const endpoint = process.env.NEXT_PUBLIC_API_URL;

const fetchWithToken = async (url, token) => {

  return fetch(url, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    }
  }).then(res => {
    if (!res.ok) {
      throw new Error('An error occurred while fetching the data.');
    }
    return res.json();
  });
};

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

export function useUser(token) {
  const { data, error, isLoading } = useSWR(token ? [`${endpoint}/auth/github/user`, token] : null, ([url, token]) => fetchWithToken(url, token), {
    dedupingInterval: 300000, // The request will not be repeated for 5 minutes
  })
  return {
    user: data,
    isLoading,
    isError: error,
  }
}

export function useTreeCommitInfo(path) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/tree/commit-info?path=${path}`, fetcher, {
    dedupingInterval: 300000,
  })
  return {
    tree: data,
    isTreeLoading: isLoading,
    isTreeError: error,
  }
}

export function useBlobContent(path) {
  const { data, error, isLoading } = useSWR(`${endpoint}/api/v1/blob?path=${path}`, fetcher, {
    dedupingInterval: 300000,
  })
  return {
    blob: data,
    isBlobLoading: isLoading,
    isBlobError: error,
  }
}
