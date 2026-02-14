import { useEffect, useMemo, useRef } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Card, Box, Heading, Text, Flex } from "@radix-ui/themes";
import { z } from "zod";
import { getAuth, setCredentials } from "../auth";

export const SearchParams = z.object({
  redirect: z.string().optional(),
});

export const Route = createFileRoute("/login")({
  component: LoginPage,
  head: () => ({
    meta: [{ title: "Login | Zap" }],
  }),
  validateSearch: SearchParams,
  beforeLoad: async ({ search }) => {
    // If already authenticated, redirect immediately
    const auth = await getAuth();
    if (auth) {
      window.location.href =
        search.redirect && search.redirect.startsWith("/") ? search.redirect : "/";
    }
  },
});

function LoginPage() {
  const { redirect } = Route.useSearch();
  const widgetRef = useRef<HTMLDivElement>(null);

  const checkedRedirect = useMemo(() => {
    if (redirect && redirect.startsWith("/")) {
      return redirect;
    }
    return "/";
  }, [redirect]);

  useEffect(() => {
    const botUsername = process.env.TG_BOT_USERNAME;
    if (!botUsername) {
      console.error("TG_BOT_USERNAME is not set");
      return;
    }

    // Define the global callback for the Telegram Login Widget
    (window as any).onTelegramAuth = (user: any) => {
      const b64 = btoa(JSON.stringify(user));
      setCredentials({ token: `tg_data_check:${b64}` });
      window.location.href = checkedRedirect;
    };

    // Inject the Telegram Login Widget script
    const container = widgetRef.current;
    if (!container) return;

    const script = document.createElement("script");
    script.src = "https://telegram.org/js/telegram-widget.js?22";
    script.async = true;
    script.setAttribute("data-telegram-login", botUsername);
    script.setAttribute("data-size", "large");
    script.setAttribute("data-onauth", "onTelegramAuth(user)");
    script.setAttribute("data-request-access", "write");
    container.appendChild(script);

    return () => {
      delete (window as any).onTelegramAuth;
      if (container.contains(script)) {
        container.removeChild(script);
      }
    };
  }, [checkedRedirect]);

  return (
    <Box p="8">
      <Card>
        <Flex m="4" direction="column" gap="3">
          <Heading>Sign in to Zap</Heading>
          <Text>Use your Telegram account to continue:</Text>
          <Box mt="2" ref={widgetRef} />
        </Flex>
      </Card>
    </Box>
  );
}
