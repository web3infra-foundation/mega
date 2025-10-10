import { useMemo, useState } from 'react'
import Head from 'next/head'

import { Theme } from '@radix-ui/themes'
import { Button, UIText } from '@gitmono/ui'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { MonoTagList } from '@/components/CodeView/Tags/MonoTagList'
import NewMonoTagDialog from '@/components/CodeView/Tags/NewMonoTagDialog'
import { IndexPageContainer, IndexPageContent, IndexPageEmptyState, IndexPageLoading, IndexSearchInput } from '@/components/IndexPages/components'
import { usePostMonoTagList } from '@/hooks/usePostMonoTagList'
import { useQueryClient } from '@tanstack/react-query'

function CodeTagsPage() {
  const [q, setQ] = useState('')
  const [dialogOpen, setDialogOpen] = useState(false)

  const queryClient = useQueryClient()
  const { data, isLoading, isFetching, refetch } = usePostMonoTagList({
    additional: '/',
    pagination: { page: 1, per_page: 200 }
  })

  const [localTags, setLocalTags] = useState<any[]>([])
  const tags = useMemo(() => {
    // Merge locally created tags and API returned tags
    const apiTags = data?.data?.items ?? []
    // Only keep locally created tags not returned by API
    const localOnly = localTags.filter(t => !apiTags.some(at => at.name === t.name))
    // Only keep names of locally created tags
    const all = [...localOnly, ...apiTags]

    return all
  }, [data, localTags])

  const filtered = useMemo(() => {
    const term = q.trim().toLowerCase()

    if (!term) return tags
    return tags.filter((t) => t.name.toLowerCase().includes(term))
  }, [q, tags])

  const hasTags = filtered.length > 0

  // Force refresh API data and clear local cache after deleting a tag
  const handleDeleteTag = () => {
    setLocalTags([])
    // Force refresh API data
    queryClient.invalidateQueries({ queryKey: ['postApiTagsList'] })
    refetch()
  }

  // Cache locally after creating a tag, deduplicate after API returns
  const handleCreateTag = () => {
    setLocalTags([])
    // Force refresh API data
    queryClient.invalidateQueries({ queryKey: ['postApiTagsList'] })
    refetch()
  }

  return (
    <>
      <Head>
        <title>Tags</title>
      </Head>

      <Theme>
        <IndexPageContainer>
          <div className='flex items-center justify-between px-3 py-3'>
            <div className='flex items-center gap-2'>
                <UIText weight='font-semibold'>Tags</UIText>
              </div>
            <div className='flex items-center gap-2'>
              <IndexSearchInput query={q} setQuery={setQ} isSearchLoading={isFetching} />
              <Button onClick={() => setDialogOpen(true)}>New tag</Button>
            </div>
          </div>

          <IndexPageContent>
            {(isLoading || isFetching) && <IndexPageLoading />}
            {!isLoading && !hasTags && <EmptyState />}
            {!isLoading && hasTags && <MonoTagList tags={filtered} onDelete={handleDeleteTag} />}
          </IndexPageContent>
        </IndexPageContainer>
        <NewMonoTagDialog open={dialogOpen} onOpenChange={setDialogOpen} onCreated={handleCreateTag} />
      </Theme>
    </>
  )
}

function EmptyState() {
  return (
    <IndexPageEmptyState>  
      <div className='flex flex-col gap-1'>
        <UIText size='text-base' weight='font-semibold'>No tags yet</UIText>
      </div>
    </IndexPageEmptyState>
  )
}

CodeTagsPage.getProviders = (
  page: React.ReactNode,
  pageProps: React.JSX.IntrinsicAttributes & { children?: React.ReactNode | undefined }
) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default CodeTagsPage
