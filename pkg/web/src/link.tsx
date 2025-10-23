import React from "react";
import { createLink, type LinkComponent } from "@tanstack/react-router";
import * as RadixThemes from "@radix-ui/themes";

const RadixLinkComponent = React.forwardRef<
  HTMLAnchorElement,
  React.ComponentProps<typeof RadixThemes.Link>
>((props, ref) => {
  return <RadixThemes.Link ref={ref} {...props} />;
});

RadixLinkComponent.displayName = "RadixLinkComponent";

const CreatedLinkComponent = createLink(RadixLinkComponent);

export const Link: LinkComponent<typeof RadixLinkComponent> = (props) => {
  return <CreatedLinkComponent preload="intent" {...props} />;
};
