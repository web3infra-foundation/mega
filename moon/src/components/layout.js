import Navbar from './TopNavbar'
import Footer from './Bottombar'
import '../styles/index.css';

export default function Layout({ children }) {
    return (
        <>
            <Navbar />
            <main>{children}</main>
            <Footer />
        </>
    )
}