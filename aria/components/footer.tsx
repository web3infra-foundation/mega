import Link from "next/link";
import { HeartIcon, TriangleIcon } from "lucide-react";

import { buttonVariants } from "@/components/ui/button";
import { Logo } from "@/components/navbar";

export function Footer() {
  return (
    <footer className="border-t w-full h-16">
      <div className="container flex items-center sm:justify-between justify-center sm:gap-0 gap-4 h-full text-muted-foreground text-sm flex-wrap sm:py-0 py-3 max-sm:px-4">
        <div className="flex items-center gap-3">
          {/*<CommandIcon className="sm:block hidden w-5 h-5 text-muted-foreground" />*/}
            <Logo />
          <p className="text-center">
            Build by{" "}
            <Link
              className="px-1 underline underline-offset-2"
              href="https://github.com/web3infra-foundation"
            >
              Web3 Infrastructure Foundation
            </Link>
            . The source code is available on{" "}
            <Link
              className="px-1 underline underline-offset-2"
              href="https://github.com/web3infra-foundation/mega"
            >
              GitHub
            </Link>
            . If you have anything else you want to ask,
            <Link
              className="px-1 underline underline-offset-2"
              href="https://calendar.app.google/QuBf2sdmf68wVYWL7"
            >
            reach out to us
          </Link>
          </p>
        </div>

        {/*<div className="gap-4 items-center hidden md:flex">*/}
          {/*<FooterButtons />*/}
        {/*</div>*/}
      </div>
    </footer>
  );
}

export function FooterButtons() {
  return (
    <>
      <Link
        href="#"
        className={buttonVariants({ variant: "outline", size: "sm" })}
      >
        <TriangleIcon className="h-[0.8rem] w-4 mr-2 text-primary fill-current" />
        Deploy
      </Link>
      <Link
        href="#"
        className={buttonVariants({ variant: "outline", size: "sm" })}
      >
        <HeartIcon className="h-4 w-4 mr-2 text-red-600 fill-current" />
        Sponsor
      </Link>
    </>
  );
}
