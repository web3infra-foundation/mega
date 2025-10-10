import { useRouter } from 'next/router'
import Head from 'next/head'
import { Theme } from '@radix-ui/themes'
import { Button, UIText } from '@gitmono/ui'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetMonoTag } from '@/hooks/useGetMonoTag'
import { useDeleteMonoTag } from '@/hooks/useDeleteMonoTag'

function CodeTagDetailPage() {
  const router = useRouter()
  const name = router.query.name as string
  const { data, isLoading } = useGetMonoTag(name, undefined, !!name)
  const del = useDeleteMonoTag()

  return (
    <>
      <Head>
        <title>Tag {name}</title>
      </Head>
      <Theme>
        <div className='m-4 rounded-md bg-white p-4'>
          <div className='mb-4 flex items-center justify-between'>
            <div className='flex items-center gap-2'>
              <UIText weight='font-semibold'>{name}</UIText>
            </div>
            <Button variant='destructive' onClick={() => del.mutate(name)}>
              Delete
            </Button>
          </div>
          {!isLoading && data?.data && (
            <div className='grid grid-cols-2 gap-4'>
              <InfoItem label='Object type' value={data.data.object_type} />
              <InfoItem label='Object ID' value={data.data.object_id} />
              <InfoItem label='Tagger' value={data.data.tagger} />
              <InfoItem label='Created at' value={data.data.created_at} />
              <div className='col-span-2'>
                <UIText quaternary className='mb-1'>Message</UIText>
                <div className='rounded-md border p-2 text-sm'>{data.data.message || '-'}</div>
              </div>
            </div>
          )}
        </div>
      </Theme>
    </>
  )
}

function InfoItem({ label, value }: { label: string; value?: string | null }) {
  return (
    <div>
      <UIText quaternary className='mb-1'>
        {label}
      </UIText>
      <div className='rounded-md border p-2 text-sm'>{value || '-'}</div>
    </div>
  )
}

CodeTagDetailPage.getProviders = (
  page: React.ReactNode,
  pageProps: React.JSX.IntrinsicAttributes & { children?: React.ReactNode | undefined }
) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default CodeTagDetailPage
