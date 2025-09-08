import { createFileRoute } from "@tanstack/react-router";
import { Flex, Card, Heading } from "@radix-ui/themes";
import { $api } from "../../api";
import { queryDevicesList } from "@/api_client/@tanstack/react-query.gen";

export const Route = createFileRoute("/_app/leaderboard")({
  component: ZapPage,
  head: () => ({
    meta: [{ title: "Leaderboard | Zap" }],
  }),
});

function ZapPage() {
  // Get the leaders.
  const { data: leaders } = $api.useSuspenseQuery(queryDevicesList);

  return (
    <Flex style={{ flex: 1 }} direction="column" gap="2" p="4">
      <Heading>Leaderboard</Heading>
      {leaders && <Card>Found {leaders.length} devices</Card>}
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
      <Card>pad</Card>
    </Flex>
  );
}
