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

// const fetcher = async url => {
//   const res = await fetch(url)

//   if (!res.ok) {
//     const error = new Error('An error occurred while fetching the data.')
//     error.info = await res.json()
//     error.status = res.status
//     throw error
//   }
//   return res.json()
// }

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