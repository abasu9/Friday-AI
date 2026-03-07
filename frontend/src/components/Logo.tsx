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
              src="/friday-mark.svg"
              alt="Friday logo"
              width={40}
              height={40}
              className="h-10 w-10 rounded-xl object-contain"
            />
          </button>
        </DialogTrigger>
      ) : (
        <DialogTrigger asChild>
          <button className="mb-2 flex cursor-pointer items-center justify-center rounded-2xl border border-border bg-card p-2 hover:opacity-80 transition-opacity">
            <Image
              src="/friday-mark.svg"
              alt="Friday logo"
              width={64}
              height={64}
              className="h-16 w-16 rounded-[1.35rem] object-contain"
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
