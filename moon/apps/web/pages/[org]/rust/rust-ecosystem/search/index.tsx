import Head from 'next/head'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useState, useEffect, useCallback } from 'react'
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline'
import { useRouter } from 'next/router'

interface SearchItem {
  name: string
  version: string
  date: string
  nsfront: string
  nsbehind: string
}

interface QueryData {
  total_page: number
  items: SearchItem[]
}

interface SearchData {
  code: number
  message: string
  data: QueryData
}

export default function SearchResultsPage() {
  const [search, setSearch] = useState('')
  const [activeTab, setActiveTab] = useState('All')
  const [searchResults, setSearchResults] = useState<SearchItem[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [totalResults, setTotalResults] = useState(0)
  const [currentPage, setCurrentPage] = useState(1)
  const [totalPages, setTotalPages] = useState(1)
  const router = useRouter()

  const performSearch = useCallback(async (query: string, page: number = 1) => {
    if (!query.trim()) {
      setSearchResults([])
      setTotalResults(0)
      setTotalPages(1)
      return
    }

    try {
      setLoading(true)
      setError(null)
      const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL
      
      const response = await fetch(`${apiBaseUrl}/api/search`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          query: query.trim(),
          pagination: {
            page: page,
            per_page: 20
          }
        })
      })

      if (!response.ok) {
        throw new Error('Failed to fetch search results')
      }
      
      const data: SearchData = await response.json()
      
      if (data.code === 200) {
        setSearchResults(data.data.items || [])
        setTotalResults(data.data.items.length)
        setTotalPages(data.data.total_page || 1)
        setCurrentPage(page)
      } else {
        throw new Error(data.message || '搜索失败')
      }
    } catch (err) {
      setError('搜索失败，请稍后重试')
      setSearchResults([])
      setTotalResults(0)
      setTotalPages(1)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    const { q } = router.query

    if (q) {
      setSearch(q as string)
      performSearch(q as string)
    }
  }, [router.query, performSearch, search])

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    if (search.trim()) {
      router.push({
        pathname: router.pathname,
        query: { ...router.query, q: search.trim() }
      })
      performSearch(search.trim())
    }
  }

  // 当activeTab改变时重新搜索
  useEffect(() => {
    if (search.trim()) {
      setCurrentPage(1) // 重置页码
      performSearch(search.trim(), 1) // 切换标签时重置到第一页
    }
  }, [activeTab, performSearch, search])

  // 分页处理函数
  const handlePreviousPage = () => {
    if (currentPage > 1) {
      performSearch(search.trim(), currentPage - 1)
    }
  }

  const handleNextPage = () => {
    if (currentPage < totalPages) {
      performSearch(search.trim(), currentPage + 1)
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
      <div className="h-screen flex flex-col">
        {/* 搜索栏 - 固定在顶部 */}
        <div className="w-full flex justify-center flex-shrink-0" style={{ background: '#FFF' }}>
          <div
            className="flex items-center"
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
              <div className="relative ml-0 mt-0">
                <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none" style={{ transform: 'translate(20px, 1px)' }}>
                  <MagnifyingGlassIcon className="h-5 w-5 text-gray-400" />
                </div>
                <input
                  type="text"
                  placeholder="Search..."
                  className="block w-full pl-10 pr-3 py-2 border-0 focus:ring-0 focus:outline-none bg-transparent text-gray-900 placeholder-gray-500"
                  style={{ transform: 'translate(20px, 1px)' }}
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
            
        {/* 分类标签 - 固定在搜索栏下方 */}
        <div className="w-full flex justify-center flex-shrink-0" style={{ background: '#F4F4F5' }}>
          <div style={{ width: '1370px', paddingLeft: 32, paddingRight: 32, paddingTop: 24 }}>
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
          </div>
        </div>

        {/* 可滚动内容区域 */}
        <div className="flex-1 overflow-auto" style={{ background: '#F4F4F5' }}>
          <div className="w-full flex justify-center pb-8">
            <div style={{ width: '1370px', paddingLeft: 32, paddingRight: 32, paddingTop: 24 }}>
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
                    Total {totalResults} results
                  </span>
                </div>

                {/* 加载状态 */}
                {loading && (
                  <div className="flex justify-center items-center py-8">
                    <div className="text-gray-500">搜索中...</div>
                  </div>
                )}
                
                {/* 错误状态 */}
                {error && (
                  <div className="flex justify-center items-center py-8">
                    <div className="text-red-500">{error}</div>
                  </div>
                )}
                
                {/* 搜索结果列表 */}
                {!loading && !error && searchResults.length > 0 && (
                  <div className="space-y-0">
                    {searchResults.map((item, index) => (
                      <div
                        key={`${item.name}-${item.version}-${item.nsfront}-${item.nsbehind}`}
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
                            {item.nsfront}/{item.nsbehind}
                          </span>
                          <h3 
                            className="text-lg font-medium text-blue-600 hover:text-blue-800 cursor-pointer"
                            onClick={() => router.push({
                              pathname: `/${router.query.org}/rust/rust-ecosystem/crate-info/`,
                              query: { 
                                crateName: item.name,
                                version: item.version || '1.0.0',
                                nsfront: item.nsfront,
                                nsbehind: item.nsbehind
                              }
                            })}
                          >
                            {item.name}
                          </h3>
                          <p className="text-sm text-gray-500">
                            Version {item.version} {item.date && `• Published ${item.date}`}
                          </p>
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {/* 无结果状态 */}
                {!loading && !error && searchResults.length === 0 && search.trim() && (
                  <div className="flex justify-center items-center py-8">
                    <div className="text-gray-500">未找到相关结果</div>
                  </div>
                )}

                {/* 分页 - 只在有结果时显示 */}
                {!loading && !error && searchResults.length > 0 && totalPages > 1 && (
                  <div className="flex justify-center items-center space-x-4 mt-8 mb-8">
                    <button 
                      onClick={handlePreviousPage}
                      disabled={currentPage <= 1}
                      className="flex items-center text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                      </svg>
                      Previous
                    </button>
                    <span className="text-lg font-bold text-gray-900">{currentPage} / {totalPages}</span>
                    <button 
                      onClick={handleNextPage}
                      disabled={currentPage >= totalPages}
                      className="flex items-center text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Next
                      <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                      </svg>
                    </button>
                  </div>
                )}
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

