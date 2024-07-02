import '../styles/TopNavbar.css';
import Link from 'next/link';

const TopNavbar = () => {
    return (
        <nav>
            <div className='navContainer'>
                {/* level first */}
                <div className='logo-container'>
                    <div className='navLeft'>
                        <Link href='/'><img src="/images/megaTitle.png" alt="logo" className='logOfMega'></img></Link>
                    </div>
                    <div className='navRight'>
                        <div>
                            <div className="navSearchContainer">
                                <div className="navSearchNextContainer">
                                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth="1.5" stroke="currentColor" className="navSearchContainerSv">
                                        <path strokeLinecap="round" strokeLinejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
                                    </svg>
                                </div>
                                <input
                                    type="email"
                                    name="email"
                                    id="email"
                                    className="navSerchInput"
                                />
                            </div>
                        </div>
                        <a href='#'><img className='logOfNotification navLogs navRightItem' src='/icons/notification.svg' ></img></a>
                        <a href='#'><img className='navLogs navRightItem' src='/icons/help.svg' ></img></a>
                        <a href='#'><img className='logOfHead navRightItem' src='/icons/headImage.svg' ></img></a>
                    </div>
                </div>
                {/* link button */}
                <div className='navLinkIcons'>
                    <ul className='navLinkUl'>
                        <li className='navLinkLi'><img className='navPagesIcon' src="/icons/code.svg"></img><Link href='/'>Code</Link></li>
                        <li className='navLinkLi'><img className='navPagesIcon ' src="/icons/issues.svg"></img><Link href='/issue'>Issues</Link></li>
                        <li className='navLinkLi'><img className='navPagesIcon ' src="/icons/git-pull-request.svg"></img><Link href='/mr'>Merge requests</Link></li>
                    </ul>
                </div>
            </div >
        </nav >
    );
};

export default TopNavbar;
