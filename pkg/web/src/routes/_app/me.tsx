import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { Box, Card, Flex, Heading, Text, Button, Avatar } from "@radix-ui/themes";
import { $api } from "../../api";
import { queryAuthMe, queryAuthGetUser } from "../../api_client/@tanstack/react-query.gen";
import type { User } from "../../api_client/types.gen";
import { removeCredentials } from "../../auth";

export const Route = createFileRoute("/_app/me")({
  component: MePage,
  head: () => ({
    meta: [{ title: "Me | Zap" }],
  }),
});

function MePage() {
  const navigate = useNavigate();

  // Get the current invoker
  const { data: invoker } = $api.useSuspenseQuery(queryAuthMe);

  // Extract user ID from the invoker
  const userId = "User" in invoker ? invoker.User.id : null;

  // Get full user details
  const { data: user } = $api.useSuspenseQuery(queryAuthGetUser as any, {
    path: { id: userId! },
  }) as { data: User };

  return (
    <Box p="4" style={{ width: "100%" }}>
      <Flex direction="column" gap="4">
        <Heading size="5">Profile</Heading>

        <Card>
          <Flex gap="4" align="center" p="2">
            <Avatar
              size="5"
              radius="full"
              fallback={user.name.charAt(0).toUpperCase()}
              src={user.photoUrl ?? undefined}
            />
            <Flex direction="column" gap="1">
              <Text size="4" weight="bold">
                {user.name}
              </Text>
              <Text size="2" color="gray">
                {user.id}
              </Text>
            </Flex>
          </Flex>
        </Card>

        <Button
          color="red"
          variant="soft"
          onClick={() => {
            removeCredentials();
            navigate({ to: "/login" });
          }}
        >
          Log out
        </Button>
      </Flex>
    </Box>
  );
}
