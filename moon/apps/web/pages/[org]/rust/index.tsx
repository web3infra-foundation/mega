import Head from 'next/head'
import { useRouter } from 'next/router'
// import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'
import { IndexSearchInput } from '@/components/IndexPages/components'
import { useState } from 'react'
import Image from 'next/image'

const Logo = () => (
  <div className="flex items-center">
    <Image src="/rust/logo.png" alt="Rust Logo" width={24} height={24} className="mr-2 relative" style={{ opacity: 0.8, top: '-2px' }} />
  </div>
)



const CratesproPage: PageWithLayout<any> = () => {
  const router = useRouter()
  const [query, setQuery] = useState('')
  const isSearchLoading = false // 如有异步搜索可改为实际loading


  const handleNavClick = (navKey: string) => {
      const org = router.query.org ? `/${router.query.org}` : ''
      
    switch (navKey) {
      case 'crates':
      router.push(`${org}/rust/rust-ecosystem`)
        break
      case 'news':
        router.push(`${org}/rust/rust-news`)
        break
      case 'cves':
        router.push(`${org}/rust/rust-ecosystem/ecosystem-cve`)
        break
      case 'releases':
        // 暂时没有链接页面
        alert('Releases 页面暂未开放')
        break
      default:
        break
    }
  }

  return (
    <>
      <Head>
        <title>Rust Ecosystem</title>
      </Head>
      <div className="flex flex-col min-h-screen w-full bg-white">
        {/* 顶部导航栏 */}
        <div className="w-full flex flex-col">
          <div className="flex items-center px-8 pt-8 pb-4 w-full">
            {/* 左侧：Logo + 导航按钮 */}
            <div className="flex items-center gap-8">
              <Logo />
              
              {/* 导航按钮 */}
              <div className="flex items-center gap-8">
                <button
                  onClick={() => handleNavClick('crates')}
                  className="text-gray-700 hover:text-blue-600 font-medium transition-colors"
                >
                  Crates
                </button>
                <button
                  onClick={() => handleNavClick('news')}
                  className="text-gray-700 hover:text-blue-600 font-medium transition-colors"
                >
                  News
                </button>
                <button
                  onClick={() => handleNavClick('cves')}
                  className="text-gray-700 hover:text-blue-600 font-medium transition-colors"
                >
                  CVEs
                </button>
                <button
                  onClick={() => handleNavClick('releases')}
                  className="text-gray-700 hover:text-blue-600 font-medium transition-colors"
                >
                  Releases
                </button>
              </div>
            </div>

            {/* 中间：搜索栏 */}
            <div className="flex-1 flex justify-center ml-38">
              <div className="relative max-w-xl w-full">
                <div className="flex items-center gap-2 px-4 py-2 border rounded-full bg-white shadow-sm" style={{ borderColor: 'rgb(253,236,231)' }}>
                  <IndexSearchInput query={query} setQuery={setQuery} isSearchLoading={isSearchLoading} />
                </div>
              </div>
            </div>

            {/* 右侧：空白区域保持平衡 */}
            <div className="w-0 flex-1"></div>
          </div>
          <div className="border-b border-gray-200 w-full" />
        </div>

        {/* 主体内容区域 - 2x2 网格布局 */}
        <div className="max-w-7xl mx-auto w-full px-8">
          {/* 主标题 - 左对齐，跨越整个宽度 */}
          <h1 className="text-3xl sm:text-4xl font-bold mb-8 text-left">Rust Ecosystem Updates</h1>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
            {/* 左侧列 */}
            <div className="space-y-8">
              {/* Latest Crate Updates */}
              <section className="flex flex-col">
                <h2 className="text-2xl font-bold mb-4 pb-2 border-b" style={{ borderColor: 'rgb(253,236,231)' }}>Latest Crate Updates</h2>
                <div className="flex-grow overflow-x-auto rounded-lg border bg-white shadow-sm" style={{ borderColor: 'rgb(253,236,231)' }}>
                  <table className="w-full text-left h-full">
                    <thead style={{ backgroundColor: 'rgb(253,236,231)' }}>
                      <tr>
                        <th className="px-6 py-3 text-sm font-semibold">Crate</th>
                        <th className="px-6 py-3 text-sm font-semibold">Version</th>
                        <th className="px-6 py-3 text-sm font-semibold">Updated</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y" style={{ borderColor: 'rgb(253,236,231)' }}>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">serde</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">1.0.197</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-26</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">tokio</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">1.38.1</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-25</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">actix-web</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">4.8.0</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-24</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">reqwest</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">0.12.5</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-23</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </section>

              {/* Latest Rust CVEs */}
              <section className="flex flex-col">
                <h2 className="text-2xl font-bold mb-4 pb-2 border-b" style={{ borderColor: 'rgb(253,236,231)' }}>Latest Rust CVEs</h2>
                <div className="flex-grow overflow-x-auto rounded-lg border bg-white shadow-sm" style={{ borderColor: 'rgb(253,236,231)' }}>
                  <table className="w-full text-left h-full">
                    <thead style={{ backgroundColor: 'rgb(253,236,231)' }}>
                      <tr>
                        <th className="px-6 py-3 text-sm font-semibold">CVE ID</th>
                        <th className="px-6 py-3 text-sm font-semibold">Description</th>
                        <th className="px-6 py-3 text-sm font-semibold">Severity</th>
                        <th className="px-6 py-3 text-sm font-semibold">Date</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y" style={{ borderColor: 'rgb(253,236,231)' }}>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">CVE-2024-1234</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Memory safety issue in crate X</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">High</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-27</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">CVE-2024-5678</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Denial of service in crate Y</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Medium</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-26</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">CVE-2024-9012</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Information leak in crate Z</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Low</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-25</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">CVE-2024-4321</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Integer overflow in crate A</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Medium</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-24</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </section>
            </div>

            {/* 右侧列 */}
            <div className="space-y-8">
              {/* Latest Rust News */}
              <section className="flex flex-col">
                <h2 className="text-2xl font-bold mb-4 pb-2 border-b" style={{ borderColor: 'rgb(253,236,231)' }}>Latest Rust News</h2>
                <div className="flex-grow overflow-x-auto rounded-lg border bg-white shadow-sm" style={{ borderColor: 'rgb(253,236,231)' }}>
                  <table className="w-full text-left h-full">
                    <thead style={{ backgroundColor: 'rgb(253,236,231)' }}>
                      <tr>
                        <th className="px-6 py-3 text-sm font-semibold">Title</th>
                        <th className="px-6 py-3 text-sm font-semibold">Source</th>
                        <th className="px-6 py-3 text-sm font-semibold">Date</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y" style={{ borderColor: 'rgb(253,236,231)' }}>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">Rust 2024 Edition Released</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Rust Blog</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-27</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">Async Rust Improvements</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">This Week in Rust</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-26</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">New Web Framework Announced</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Rust Community Forum</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-25</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">Exploring the future of unsafe Rust</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Rust Blog</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-24</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </section>

              {/* Latest Rust Releases */}
              <section className="flex flex-col">
                <h2 className="text-2xl font-bold mb-4 pb-2 border-b" style={{ borderColor: 'rgb(253,236,231)' }}>Latest Rust Releases</h2>
                <div className="flex-grow overflow-x-auto rounded-lg border bg-white shadow-sm" style={{ borderColor: 'rgb(253,236,231)' }}>
                  <table className="w-full text-left h-full">
                    <thead style={{ backgroundColor: 'rgb(253,236,231)' }}>
                      <tr>
                        <th className="px-6 py-3 text-sm font-semibold">Version</th>
                        <th className="px-6 py-3 text-sm font-semibold">Date</th>
                        <th className="px-6 py-3 text-sm font-semibold">Notes</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y" style={{ borderColor: 'rgb(253,236,231)' }}>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">Rust 1.79.0</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-20</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Stable release</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">Rust 1.80.0-beta.1</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-13</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Beta release</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">Rust 1.78.1</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-06</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Patch release</td>
                      </tr>
                      <tr>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">Rust 1.78.0</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-06-29</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">Stable release</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </section>
            </div>
          </div>
        </div>
      </div>
    </>
  )
}

CratesproPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default CratesproPage 