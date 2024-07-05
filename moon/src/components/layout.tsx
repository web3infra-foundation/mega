import Navbar from './TopNavBar'
import Footer from './BottomBar'

export default function Layout({ children }) {
    return (
        <div className="flex flex-col min-h-screen">
            <Navbar />
            <main className="flex-grow">
                {children}
            </main>
            <Footer />
        </div>
    )
}