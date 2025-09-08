import { Outlet, createFileRoute } from "@tanstack/react-router";
import { Box, Flex, Button, Card, Text, Spinner } from "@radix-ui/themes";
import { Suspense } from "react";
import { Link } from "../link";
import { LightningIcon, RankingIcon } from "@phosphor-icons/react";
import { checkAuthBeforeLoad } from "../auth";
import { ApiProvider } from "../api";

export const Route = createFileRoute("/_app")({
  component: AppLayout,
  beforeLoad: checkAuthBeforeLoad,
});

function AppLayout() {
  return (
    <>
      <Flex
        style={{
          minHeight: "100dvh",
          paddingBottom: "5.5rem",
        }}
      >
        <ApiProvider>
          <Suspense
            fallback={
              <Flex
                justify="center"
                align="center"
                style={{ flex: 1 }}
              >
                <Spinner size="3" />
              </Flex>
            }
          >
            <Outlet />
          </Suspense>
        </ApiProvider>
      </Flex>
      <AppBar />
    </>
  );
}

function AppBar() {
  return (
    <Flex
      style={{
        position: "fixed",
        zIndex: 1,
        bottom: 0,
        left: 0,
        right: 0,
      }}
      justify="center"
      p="3"
      pb="6"
    >
      <Card
        variant="classic"
        style={{
          boxShadow: "var(--shadow-6)",
          backgroundColor: "var(--gray-0)",
          backdropFilter: "none",
        }}
      >
        <Flex gap="3" align="center">
          <Box px="2">
            <Text color="yellow" size="5" weight="bold">
              zap
            </Text>
          </Box>
          <NavButton to="/zap">
            <LightningIcon />
            shock
          </NavButton>
          <NavButton to="/leaderboard">
            <RankingIcon />
            leaders
          </NavButton>
        </Flex>
      </Card>
    </Flex>
  );
}

function NavButton({
  to,
  children,
}: {
  to: React.ComponentProps<typeof Link>["to"];
  children: React.ReactNode;
}) {
  return (
    <Link to={to} asChild>
      {({ isActive }) =>
        isActive ? (
          <Button variant="solid">{children}</Button>
        ) : (
          <Button variant="soft">{children}</Button>
        )
      }
    </Link>
  );
}
