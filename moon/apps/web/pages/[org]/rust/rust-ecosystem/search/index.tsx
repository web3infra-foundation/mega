import Head from 'next/head'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useState, useEffect, useCallback } from 'react'
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline'
import { useRouter } from 'next/router'

// 模拟搜索结果数据 - 移到组件外部
const mockSearchResults = [
    {
      id: 1,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 2,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 3,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 4,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 5,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 6,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 7,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 8,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 9,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    },
    {
      id: 10,
      type: 'NuGet',
      name: 'Text',
      details: 'Package 1.0.6 Published October 22, 2021',
      description: 'A simple text processing library for .NET applications',
      version: '1.0.6',
      publishedDate: '2021-10-22',
      downloads: 15420,
      stars: 45
    }
  ]

export default function SearchResultsPage() {
  const [search, setSearch] = useState('')
  const [activeTab, setActiveTab] = useState('All')
  const [searchResults, setSearchResults] = useState<any[]>([])
  const [loading, setLoading] = useState(false)
  const router = useRouter()

  const performSearch = useCallback(async (_query: string) => {
    setLoading(true)
    setTimeout(() => {
      setSearchResults(mockSearchResults)
      setLoading(false)
    }, 500)
  }, [])

  useEffect(() => {
    const { q } = router.query

    if (q) {
      setSearch(q as string)
      performSearch(q as string)
    }
  }, [router.query, performSearch])

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    if (search.trim()) {
      router.push({
        pathname: router.pathname,
        query: { ...router.query, q: search.trim() }
      })
    }
  }

  const tabs = [
    { id: 'All', label: 'All' },
    { id: 'Packages', label: 'Packages' },
    { id: 'Advisories', label: 'Advisories' },
    { id: 'Projects', label: 'Projects' }
  ]

  return (
    <>
      <Head>
        <title>Search Results - Rust Ecosystem</title>
      </Head>
                             <div className="min-h-screen  w-full bg-white">
                   {/* 搜索栏 */}
                     <div className="w-full flex justify-center mb-4" style={{ background: '#FFF' }}>
             <div
               className="flex items-center sticky top-0 z-20"
               style={{
                 width: '1680px',
                 height: '43px',
                 flexShrink: 0,
                 marginTop: 0,
                 marginBottom: 0,
                 paddingLeft: 32,
                 paddingRight: 32,
                 background: '#FFF',
                 boxSizing: 'border-box',
               }}
             >
            <form onSubmit={handleSearch} className="flex-1 max-w-xl ml-8">
              <div className="relative ml-10 mt-5">
                <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <MagnifyingGlassIcon className="h-5 w-5 text-gray-400" />
                </div>
                <input
                  type="text"
                  placeholder="Search..."
                  className="block w-full pl-10 pr-3 py-2 border-0 focus:ring-0 focus:outline-none bg-transparent text-gray-900 placeholder-gray-500"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
              </div>
            </form>
          </div>
          <div 
            style={{
              width: '1680px',
              height: '8px',
              background: '#F4F4F5',
              marginTop: 0,
              marginBottom: 0,
              paddingLeft: 32,
              paddingRight: 32,
              boxSizing: 'border-box',
            }}
          />
        </div>
            
                                   {/* 分类标签和搜索结果区域 */}
          <div className="w-full flex justify-center" style={{ background: '#F4F4F5' }}>
            <div style={{ width: '1370px', paddingLeft: 32, paddingRight: 32, paddingTop: 24 }}>
                         {/* 分类标签 */}
             <div className="flex space-x-8 mb-0">
               {tabs.map((tab) => (
                 <button
                   key={tab.id}
                   onClick={() => setActiveTab(tab.id)}
                   className={`py-2 px-1 border-b-2 transition-colors ${
                     activeTab === tab.id
                       ? 'border-blue-500'
                       : 'border-transparent hover:text-gray-700 hover:border-gray-300'
                   }`}
                   style={{
                     color: activeTab === tab.id ? '#1c2024' : '#6b7280',
                     fontFamily: '"HarmonyOS Sans SC"',
                     fontSize: '24px',
                     fontStyle: 'normal',
                     fontWeight: 500,
                     lineHeight: '20px',
                     letterSpacing: '0',
                   }}
                 >
                   {tab.label}
                 </button>
               ))}
             </div>

             {/* 搜索结果列表容器 */}
            <div style={{ 
              background: 'white', 
              borderRadius: '8px',
              boxShadow: '0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06)',
              overflow: 'hidden'
            }}>
              {/* 标题和结果数量 */}
              <div className="flex items-center justify-between p-3 border-b border-gray-200">
                <h1 
                  style={{
                    display: '-webkit-box',
                    WebkitBoxOrient: 'vertical',
                    WebkitLineClamp: 1,
                    overflow: 'hidden',
                    color: '#1c2024',
                    textOverflow: 'ellipsis',
                    fontFamily: '"HarmonyOS Sans SC"',
                    fontSize: '28px',
                    fontStyle: 'normal',
                    fontWeight: 500,
                    lineHeight: '24px',
                    letterSpacing: '0'
                  }}
                >
                  ALL
                </h1>
                <span 
                  style={{
                    display: '-webkit-box',
                    WebkitBoxOrient: 'vertical',
                    WebkitLineClamp: 1,
                    overflow: 'hidden',
                    color: '#4b68ff',
                    textAlign: 'right',
                    textOverflow: 'ellipsis',
                    fontFamily: '"SF Pro"',
                    fontSize: '16px',
                    fontStyle: 'normal',
                    fontWeight: 400,
                    lineHeight: '24px',
                    letterSpacing: '0'
                  }}
                >
                  Total 56 results
                </span>
              </div>

              {loading ? (
                <div className="flex justify-center items-center py-12">
                  <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
                </div>
              ) : (
                <div className="space-y-0">
                  {searchResults.map((item, index) => (
                    <div
                      key={item.id}
                      className="transition-colors cursor-pointer"
                                             style={{ 
                         display: 'flex',
                         minWidth: '100px',
                         minHeight: '44px',
                         padding: '8px 16px',
                         alignItems: 'center',
                         gap: '8px',
                         flex: '1 0 0',
                         alignSelf: 'stretch',
                         background: '#ffffff00',
                         borderBottom: index !== searchResults.length - 1 ? '1px solid #e5e7eb' : 'none'
                       }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = '#EBEBEB'
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = '#ffffff00'
                      }}
                    >
                    <div className="flex flex-col space-y-2">
                       <span className="text-sm text-gray-500">
                         {item.type}
                       </span>
                       <h3 
                         className="text-lg font-medium text-blue-600 hover:text-blue-800 cursor-pointer"
                         onClick={() => router.push({
                           pathname: `/${router.query.org}/rust/rust-ecosystem/search/crate-info`,
                           query: { 
                             crateName: item.name,
                             version: item.version || '1.0.0'
                           }
                         })}
                       >
                         {item.name}
                       </h3>
                       <p className="text-sm text-gray-500">
                         {item.details}
                       </p>
                     </div>
                   </div>
                 ))}
               </div>
             )}

             {/* 分页 */}
             <div className="flex justify-center items-center space-x-4 mt-8">
               <button className="flex items-center text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed" disabled>
                 <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                   <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                 </svg>
                 Previous
               </button>
               <span className="text-lg font-bold text-gray-900">1</span>
               <button className="flex items-center text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed" disabled>
                 Next
                 <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                   <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                 </svg>
               </button>
             </div>
           </div>
         </div>
       </div>
     </div>
   </>
 )
}

SearchResultsPage.getProviders = (page: any, pageProps: any) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}
