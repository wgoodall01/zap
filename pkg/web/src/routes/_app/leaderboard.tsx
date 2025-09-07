import { createFileRoute } from "@tanstack/react-router";
import { Flex, Card, Heading } from "@radix-ui/themes";
import { checkAuthBeforeLoad } from "../../auth";

export const Route = createFileRoute("/_app/leaderboard")({
  component: ZapPage,
  beforeLoad: checkAuthBeforeLoad,
  head: () => ({
    meta: [{ title: "Leaderboard | Zap" }],
  }),
});

function ZapPage() {
  return (
    <Flex style={{ flex: 1 }} direction="column" gap="2" p="4">
      <Heading>Leaderboard</Heading>
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
