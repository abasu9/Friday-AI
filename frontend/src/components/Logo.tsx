import React from "react";
import Image from "next/image";
import { Dialog, DialogContent, DialogTitle, DialogTrigger } from "./ui/dialog";
import { VisuallyHidden } from "./ui/visually-hidden";
import { About } from "./About";

interface LogoProps {
    isCollapsed: boolean;
}

const Logo = React.forwardRef<HTMLButtonElement, LogoProps>(({ isCollapsed }, ref) => {
  return (
    <Dialog aria-describedby={undefined}>
      {isCollapsed ? (
        <DialogTrigger asChild>
          <button
            ref={ref}
            className="mb-2 cursor-pointer rounded-xl border border-border bg-card p-1 hover:opacity-80 transition-opacity"
          >
            <Image
              src="/friday-logo.jpg"
              alt="Friday logo"
              width={48}
              height={26}
              className="h-auto w-12 object-contain"
            />
          </button>
        </DialogTrigger>
      ) : (
        <DialogTrigger asChild>
          <button className="mb-2 block cursor-pointer rounded-2xl border border-border bg-card p-2 hover:opacity-80 transition-opacity">
            <Image
              src="/friday-logo.jpg"
              alt="Friday logo"
              width={176}
              height={96}
              className="h-auto w-44 object-contain"
            />
          </button>
        </DialogTrigger>
      )}
      <DialogContent>
        <VisuallyHidden>
          <DialogTitle>About Friday</DialogTitle>
        </VisuallyHidden>
        <About />
      </DialogContent>
    </Dialog>
  );
});

Logo.displayName = "Logo";

export default Logo;
