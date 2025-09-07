import { HeadContent, Outlet, createRootRoute } from "@tanstack/react-router";
// import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
// import { TanstackDevtools } from "@tanstack/react-devtools";

export const Route = createRootRoute({
  component: () => (
    <>
      <HeadContent />
      <Outlet />
      {/*
      <TanstackDevtools
        config={{
          position: "bottom-right",
        }}
        plugins={[
          {
            name: "Tanstack Router",
            render: <TanStackRouterDevtoolsPanel />,
          },
        ]}
      />
      */}
    </>
  ),
  head: () => ({
    meta: [{ title: "Zap" }],
  }),
});
