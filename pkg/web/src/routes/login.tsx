import { createFileRoute } from "@tanstack/react-router";
import { Card, Box, Link, Button, Heading, Text, Flex } from "@radix-ui/themes";

export const Route = createFileRoute("/login")({
  component: ZapPage,
  head: () => ({
    meta: [{ title: "Login | Zap" }],
  }),
});

function ZapPage() {
  return (
    <Box p="8">
      <Card>
        <Flex m="4" direction="column">
          <Heading>you're not signed in!</Heading>
          <Text>Open this app from Telegram instead:</Text>
          <Box mt="4">
            <Button asChild>
              <Link href="https://t.me/ansley_bark_bot">oops, okay</Link>
            </Button>
          </Box>
        </Flex>
      </Card>
    </Box>
  );
}
