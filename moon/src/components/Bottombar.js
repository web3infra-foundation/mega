import '../styles/Bottombar.css';
import '../styles/globals.css';



const Bottombar = () => {
    const currentYear = new Date().getFullYear();
    return (

        <div className='BottombarContainer'>
            <ul className='BottombarUl'>
                <li className='BottombarItems'><img src="/images/megaLogo.png" className='BottombarItemsLog'></img>© {currentYear} MEGA</li>
                <li className='BottombarItems'><a href='#'>Privacy</a></li>
                <li className='BottombarItems'><a href='#'>Security</a></li>
                <li className='BottombarItems'><a href='#'>Contact</a></li>
                <li className='BottombarItems'><a href='#'>Docs</a></li>
            </ul>
        </div>
    );
};

export default Bottombar;