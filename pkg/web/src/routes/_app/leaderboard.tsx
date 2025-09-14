import { createFileRoute } from "@tanstack/react-router";
import { Flex, Card, Heading, Text, Badge } from "@radix-ui/themes";
import { LightningIcon, Vibrate, SpeakerHigh } from "@phosphor-icons/react";
import { $api } from "../../api";
import { queryActivityLeaderboard } from "@/api_client/@tanstack/react-query.gen";
import { Profile } from "../../components/Profile";

export const Route = createFileRoute("/_app/leaderboard")({
  component: LeaderboardPage,
  head: () => ({
    meta: [{ title: "Leaderboard | Zap" }],
  }),
});

function LeaderboardPage() {
  // Get the leaders from the activity leaderboard API
  const { data: leaderboard } = $api.useSuspenseQuery(
    queryActivityLeaderboard,
    {},
  );

  return (
    <Flex style={{ flex: 1 }} direction="column" gap="2" p="4">
      <Heading>Leaderboard</Heading>

      {leaderboard?.leaders && leaderboard.leaders.length > 0 ? (
        <Flex direction="column" gap="3">
          {leaderboard.leaders.map((leader, i) => {
            const shockCount = leader.counts.Shock || 0;
            const vibrateCount = leader.counts.Vibrate || 0;
            const beepCount = leader.counts.Beep || 0;

            return (
              <Card key={leader.user.id} variant="ghost">
                <Flex align="center" gap="3">
                  {/* Total count in yellow square box */}
                  <Flex>
                    <Flex align="baseline">
                      <Text color="gray" size="2" mr="0.2em">
                        #
                      </Text>
                      <Text
                        weight="bold"
                        size="3"
                        style={{ minWidth: 20 }}
                        as="div"
                      >
                        {i + 1}
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
                      <Flex
                        align="center"
                        gap="1"
                        style={{ color: "var(--accent-9)" }}
                      >
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
          })}
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
