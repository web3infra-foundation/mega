import { LoadingSpinner } from '@gitmono/ui'

import CodeTable from '../CodeTable'

const SpinnerTable = ({ isLoading, datasource, content }: any) => {
  return (
    <div className='relative mt-3 h-screen p-3.5'>
      {isLoading ? (
        <div className='align-center container absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 justify-center'>
          <LoadingSpinner />
        </div>
      ) : (
        <CodeTable directory={datasource} loading={isLoading} readmeContent={content} />
      )}
    </div>
  )
}

export default SpinnerTable
