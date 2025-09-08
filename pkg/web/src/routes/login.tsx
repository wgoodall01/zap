import { useEffect, useMemo } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Card, Box, Link, Button, Heading, Text, Flex } from "@radix-ui/themes";
import { z } from "zod";
import { getAuth } from "../auth";

export const SearchParams = z.object({
  redirect: z.string().optional(),
});

export const Route = createFileRoute("/login")({
  component: ZapPage,
  head: () => ({
    meta: [{ title: "Login | Zap" }],
  }),
  validateSearch: SearchParams,
});

/** Interval to poll for a login that may have happened externally. */
const LOGIN_POLL_INTERVAL_MS = 300;

function ZapPage() {
  const { redirect } = Route.useSearch();

  // Make sure the redirect path starts with `/`.
  // This is a basic security measure to prevent open redirects.
  const checkedRedirect = useMemo(() => {
    if (redirect && redirect.startsWith("/")) {
      return redirect;
    }
    return "/";
  }, [redirect]);

  // Poll every 300ms to see if the user has signed in. If they have, redirect them.
  // Use setTimeout to never poll faster than `getAuth` runs.
  useEffect(() => {
    let timeout: ReturnType<typeof setTimeout> | null = null;
    async function poll() {
      const auth = await getAuth();
      if (auth) {
        window.location.href = checkedRedirect;
        return;
      }

      // Run again
      timeout = setTimeout(poll, LOGIN_POLL_INTERVAL_MS);
    }
    poll();
    return () => {
      if (timeout) clearTimeout(timeout);
    };
  }, [checkedRedirect]);

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
