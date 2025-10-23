import { createFileRoute } from "@tanstack/react-router";
import { Flex, Box, Card, Heading, Text, Badge } from "@radix-ui/themes";
import { Crown, LightningIcon, Vibrate, SpeakerHigh } from "@phosphor-icons/react";
import { $api } from "../../api";
import { queryActivityLeaderboard } from "@/api_client/@tanstack/react-query.gen";
import { Profile } from "../../components/Profile";
import { RadialLayout } from "../../components/radial_layout";

export const Route = createFileRoute("/_app/leaderboard")({
  component: LeaderboardPage,
  head: () => ({
    meta: [{ title: "Leaderboard | Zap" }],
  }),
});

function PodiumPosition({ leader, position }: { leader: any; position: 1 | 2 | 3 }) {
  const height = [
    120, // Position 1
    80, // Position 2
    50, // Position 3
  ][position - 1];

  const color = [
    "var(--accent-9)", // Position 1
    "var(--gray-6)", // Position 2
    "var(--orange-4)", // Position 3
  ][position - 1];

  return (
    <Flex direction="column" align="center" gap="2" style={{ flex: 1, minWidth: 0 }}>
      {/* User info at top */}

      <RadialLayout.Container r="20px" gap="10px">
        <RadialLayout.Centered>
          <Box
            style={{
              borderRadius: "100%",
              border: position === 1 ? "4px solid var(--accent-9)" : "4px solid transparent",
            }}
          >
            <Profile user={leader.user} size="3" />
          </Box>
        </RadialLayout.Centered>
        {position === 1 && (
          <RadialLayout.Around theta={-Math.PI / 4 - 0.2} orient>
            <Crown size={20} weight="fill" color="var(--accent-9)" />
          </RadialLayout.Around>
        )}
      </RadialLayout.Container>

      <Text size="2" weight="medium" truncate style={{ textAlign: "center", maxWidth: "100%" }}>
        {leader.user.name}
      </Text>

      {/* Podium block */}
      <Flex
        direction="row"
        justify="center"
        align="start"
        style={{
          alignSelf: "stretch",
          height: height,
          backgroundColor: color,
        }}
      >
        <Text
          size="6"
          weight="bold"
          m="1"
          style={{ color: position === 1 ? "black" : "var(--gray-a10)" }}
        >
          {position}
        </Text>
      </Flex>
    </Flex>
  );
}

function LeaderCard({ leader, position }: { leader: any; position: number }) {
  const shockCount = leader.counts.Shock || 0;
  const vibrateCount = leader.counts.Vibrate || 0;
  const beepCount = leader.counts.Beep || 0;

  return (
    <Card variant="ghost">
      <Flex align="center" gap="3">
        {/* Position number */}
        <Flex>
          <Flex align="baseline">
            <Text color="gray" size="2" mr="0.2em">
              #
            </Text>
            <Text weight="bold" size="3" style={{ minWidth: 20 }} as="div">
              {position}
            </Text>
          </Flex>
        </Flex>

        <Profile user={leader.user} size="3" />

        <Flex direction="column" style={{ flex: 1, minWidth: 0 }}>
          <Text size="3" weight="medium" truncate>
            {leader.user.name}
          </Text>
          {/* Breakdown counts under the name */}
          <Flex gap="4" align="center" style={{ opacity: 0.8 }}>
            <Flex align="center" gap="1" style={{ color: "var(--accent-9)" }}>
              <Text size="2" weight="bold">
                {shockCount}
              </Text>
              <LightningIcon weight="fill" size={14} />
            </Flex>
            <Flex align="center" gap="1">
              <Text size="2" weight="bold">
                {vibrateCount}
              </Text>
              <Vibrate size={14} />
            </Flex>
            <Flex align="center" gap="1">
              <Text size="2" weight="bold">
                {beepCount}
              </Text>
              <SpeakerHigh size={14} />
            </Flex>
          </Flex>
        </Flex>

        <Flex>
          <Badge
            size="3"
            variant="solid"
            style={{
              minWidth: 40,
              height: 40,
              fontSize: 20,
              justifyContent: "end",
            }}
          >
            <Text weight="medium">{leader.totalActions}</Text>
          </Badge>
        </Flex>
      </Flex>
    </Card>
  );
}

function LeaderboardPage() {
  // Get the leaders from the activity leaderboard API
  const { data: leaderboard } = $api.useSuspenseQuery(queryActivityLeaderboard, {});

  return (
    <Flex
      style={{ flex: 1, minWidth: 0, maxWidth: 800, margin: "0 auto" }}
      direction="column"
      gap="2"
      p="4"
    >
      <Heading>Leaderboard</Heading>

      {leaderboard?.leaders && leaderboard.leaders.length > 0 ? (
        <Flex direction="column" gap="4">
          {/* Podium for top 3 */}
          {leaderboard.leaders.length >= 3 && (
            <Flex
              justify="center"
              align="end"
              gap="2"
              style={{ alignSelf: "center", minWidth: 0, maxWidth: 400 }}
            >
              {/* Position 2 (left) */}
              <PodiumPosition leader={leaderboard.leaders[1]} position={2} />
              {/* Position 1 (center, tallest) */}
              <PodiumPosition leader={leaderboard.leaders[0]} position={1} />
              {/* Position 3 (right) */}
              <PodiumPosition leader={leaderboard.leaders[2]} position={3} />
            </Flex>
          )}

          {/* Full leaderboard list */}
          <Flex direction="column" gap="3">
            <Text size="3" weight="bold" color="gray">
              Full Rankings
            </Text>
            {leaderboard.leaders.map((leader, i) => (
              <LeaderCard key={leader.user.id} leader={leader} position={i + 1} />
            ))}
          </Flex>
        </Flex>
      ) : (
        <Card>
          <Flex p="4" justify="center">
            <Text size="2" color="gray">
              No activity data available yet.
            </Text>
          </Flex>
        </Card>
      )}
    </Flex>
  );
}
