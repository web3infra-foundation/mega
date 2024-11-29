import Image from "next/image";

import mega from '@/images/logo.svg';

export function LogoIcon(props) {
  return (
      <Image src={mega} alt="Mega" unoptimized {...props}/>
  )
}
