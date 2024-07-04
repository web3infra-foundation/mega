import Navbar from './TopNavBar'
import Footer from './Bottombar'

export default function Layout({ children }) {
    return (
        <>
            <Navbar />
            <main className='h-dvh'>{children}</main>
            <Footer />
        </>
    )
}