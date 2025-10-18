import { useState, useEffect, useCallback } from "react";
import { createFileRoute } from "@tanstack/react-router";
import {
  Button,
  Flex,
  Dialog,
  Badge,
  Switch,
  Text,
  Card,
} from "@radix-ui/themes";
import { Lightning } from "../../components/lightning";
import { RadialLayout } from "../../components/radial_layout";
import {
  LightningIcon,
  GearIcon,
  Vibrate,
  SpeakerHigh,
  Pause,
  PlugsConnected,
} from "@phosphor-icons/react";
import { type TgHaptic, playHapticFeedback } from "../../telegram";
import { $api } from "../../api";
import {
  mutationZapTrigger,
  queryZapDeviceList,
} from "../../api_client/@tanstack/react-query.gen";
import type { DeviceWithShockers } from "../../api_client/types.gen";

export const Route = createFileRoute("/_app/zap")({
  component: ZapPage,
  head: () => ({
    meta: [{ title: "ZAP ZAP" }],
  }),
});

// Duration of the effect, random distribution:
const EFFECT_DURATION_MS = 600;

function ZapPage() {
  // Fetch devices on page load
  const { data: devices } = $api.useSuspenseQuery(
    queryZapDeviceList,
    {},
    { staleTime: 5 * 60 * 1000 },
  );

  // State to trigger the lightning effect
  const [firing, setFiring] = useState(false);

  // State for shocker selection modal
  const [modalOpen, setModalOpen] = useState(false);
  const [selectedShockers, setSelectedShockers] = useState<Set<string>>(
    new Set(
      devices
        .flatMap((d) => d.shockers)
        .filter((d) => !d.isPaused)
        .map((s) => s.id),
    ),
  );

  useEffect(() => {
    if (firing) {
      const timeout = setTimeout(() => setFiring(false), EFFECT_DURATION_MS);
      return () => clearTimeout(timeout);
    }
  }, [firing]);

  // Network request to trigger shock
  const shockMutation = $api.useMutation(mutationZapTrigger);

  // Event handler: trigger shock
  const onShock = useCallback(async () => {
    if (shockMutation.isPending) return; // Prevent concurrent triggers
    if (firing) return; // Prevent too-frequent triggers

    // Start the haptics immediately.
    playZapHaptics();

    // Trigger the shock effect
    setFiring(true);

    // Send the shock request to the API using selected shockers
    if (selectedShockers.size === 0) {
      console.warn("No shockers selected");
      return;
    }

    shockMutation.mutate({
      body: {
        shockerIds: Array.from(selectedShockers),
        action: {
          Shock: {
            intensity: 50, // 50% intensity
            duration: 1000,
          },
        },
      },
    });
  }, [firing, shockMutation]);

  const onVibrate = useCallback(() => {
    if (shockMutation.isPending) return; // Prevent concurrent triggers
    playHapticFeedback({ type: "notification", style: "success" });

    shockMutation.mutate({
      body: {
        shockerIds: Array.from(selectedShockers),
        action: { Vibrate: { duration: 1000, intensity: 100 } },
      },
    });
  }, [shockMutation]);

  // Pair a shocker by sending a beep signal to it.
  const onPair = useCallback(
    (shockerId: string) => {
      if (shockMutation.isPending) return; // Prevent concurrent triggers
      shockMutation.mutate({
        body: {
          shockerIds: [shockerId],
          action: { Vibrate: { duration: 300, intensity: 10 } },
        },
      });
    },
    [shockMutation],
  );

  const onBeep = useCallback(() => {
    if (shockMutation.isPending) return; // Prevent concurrent triggers
    playHapticFeedback({ type: "notification", style: "success" });
    shockMutation.mutate({
      body: {
        shockerIds: Array.from(selectedShockers),
        action: { Beep: { duration: 400, intensity: 100 } },
      },
    });
  }, [shockMutation]);

  return (
    <Flex style={{ flex: 1 }} direction="column">
      <Lightning
        style={{
          position: "fixed",
          zIndex: -1,
          top: 0,
          left: 0,
          width: "100%",
          height: "100%",
          filter: firing ? "none" : "blur(1px) brightness(50%)",
        }}
        hue={62}
        speed={0.2}
        intensity={firing ? 1 : 0.08}
        size={0.7}
      />
      <Flex align="center" mx="auto" my="auto">
        {selectedShockers.size > 0 ? (
          <ButtonCluster
            onShock={onShock}
            onVibrate={onVibrate}
            onSound={onBeep}
            firing={firing}
          />
        ) : (
          <Card style={{ width: 250, textAlign: "center" }}>
            <Flex direction="column" m="3">
              <Text size="3" mb="4">
                No shocker channels selected. Open settings and turn one on.
              </Text>
              <Button
                variant="soft"
                size="2"
                onClick={() => setModalOpen(true)}
              >
                <GearIcon />
                Open Settings
              </Button>
            </Flex>
          </Card>
        )}
      </Flex>

      {/* Shockers button at bottom */}
      <Flex
        justify="center"
        p="4"
        style={{ position: "absolute", bottom: "6rem", left: 0, right: 0 }}
      >
        <ShockersDialog
          devices={devices}
          enabledShockers={Array.from(selectedShockers)}
          onChangeEnabled={(shockerIds) =>
            setSelectedShockers(new Set(shockerIds))
          }
          onPair={onPair}
          isOpen={modalOpen}
          onOpenChange={setModalOpen}
        />
      </Flex>
    </Flex>
  );
}

/** The central button control cluster. */
function ButtonCluster({
  onShock,
  onVibrate,
  onSound,
  firing,
}: {
  onShock: () => void;
  onVibrate: () => void;
  onSound: () => void;
  firing: boolean;
}) {
  const BUTTON_GAP_ANGLE = 0.23;

  function SatelliteButton({
    onClick,
    disabled,
    children,
  }: {
    onClick: () => void;
    disabled: boolean;
    children: React.ReactNode;
  }) {
    return (
      <Button
        size="3"
        variant="soft"
        disabled={disabled}
        style={{
          borderRadius: "100%",
          height: "3.5rem",
          width: "3.5rem",
        }}
        onClick={onClick}
      >
        {children}
      </Button>
    );
  }

  return (
    <RadialLayout.Container r="8rem" gap="2.5rem">
      <RadialLayout.Centered>
        <Button
          onClick={onShock}
          size="4"
          variant="soft"
          style={{
            fontSize: "7rem",
            width: "15rem",
            height: "15rem",
            appearance: "none",
            borderRadius: "100%",
            backdropFilter: "blur(10px)",
            border: "5px solid var(--accent-9)",
            boxShadow: ["inset", ""]
              .map((setting) => `${setting} 0 0 20px -6px var(--accent-9)`)
              .join(", "),
          }}
        >
          <LightningIcon weight={firing ? "fill" : "light"} />
        </Button>
      </RadialLayout.Centered>
      <RadialLayout.Around theta={Math.PI / 4 + BUTTON_GAP_ANGLE}>
        <SatelliteButton onClick={onVibrate} disabled={firing}>
          <Vibrate size={24} />
        </SatelliteButton>
      </RadialLayout.Around>
      <RadialLayout.Around theta={Math.PI / 4 - BUTTON_GAP_ANGLE}>
        <SatelliteButton onClick={onSound} disabled={firing}>
          <SpeakerHigh size={24} />
        </SatelliteButton>
      </RadialLayout.Around>
    </RadialLayout.Container>
  );
}

function ShockersDialog({
  devices,
  enabledShockers,
  onChangeEnabled,
  isOpen,
  onOpenChange,
  onPair,
}: {
  devices: DeviceWithShockers[];
  enabledShockers: string[];
  onChangeEnabled: (shockerIds: string[]) => void;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onPair: (shockerId: string) => void;
}) {
  const enabledSet = new Set(enabledShockers);

  return (
    <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <Dialog.Trigger>
        <Button variant="soft" size="3">
          <GearIcon />
          Channels <Badge size="2">{enabledShockers.length}</Badge>
        </Button>
      </Dialog.Trigger>
      <Dialog.Content>
        <Dialog.Title>Channels</Dialog.Title>
        <Dialog.Description size="2" mb="4">
          Choose which radio channels to activate when you press the zap button.
        </Dialog.Description>

        {devices.length === 0 ? (
          <Text size="2" color="gray">
            No devices found. Make sure you have devices configured.
          </Text>
        ) : (
          <Flex direction="column" gap="4">
            {devices.map((device) => (
              <Flex key={device.id} gap="2" direction="column">
                <Text size="3" weight="bold" mb="2" color="gray">
                  {device.name}
                </Text>
                <Flex direction="column" gap="4" ml="3">
                  {device.shockers.map((shocker) => (
                    <Flex
                      key={shocker.id}
                      gap="4"
                      style={{ opacity: shocker.isPaused ? 0.5 : 1 }}
                      align="center"
                    >
                      <Switch
                        size="3"
                        checked={enabledSet.has(shocker.id)}
                        onCheckedChange={(checked) => {
                          const newEnabled = new Set(enabledSet);
                          if (checked) {
                            newEnabled.add(shocker.id);
                          } else {
                            newEnabled.delete(shocker.id);
                          }
                          onChangeEnabled(Array.from(newEnabled));
                        }}
                        disabled={shocker.isPaused}
                      />
                      <Text
                        size="2"
                        weight="medium"
                        color={shocker.isPaused ? "gray" : undefined}
                      >
                        {shocker.name}
                      </Text>
                      <Flex ml="auto">
                        {shocker.isPaused && (
                          <Badge size="2" variant="outline">
                            <Pause />
                            Paused
                          </Badge>
                        )}
                        {!shocker.isPaused && (
                          <Button
                            variant="soft"
                            size="2"
                            onClick={() => onPair(shocker.id)}
                          >
                            <PlugsConnected />
                            Connect
                          </Button>
                        )}
                      </Flex>
                    </Flex>
                  ))}
                </Flex>
              </Flex>
            ))}
          </Flex>
        )}

        <Flex gap="3" mt="4" justify="end">
          <Dialog.Close>
            <Button variant="soft" color="gray">
              Close
            </Button>
          </Dialog.Close>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
}

async function playZapHaptics() {
  const sequence: Array<number | TgHaptic> = [
    { type: "notification", style: "warning" },
    100,
    { type: "notification", style: "error" },
  ];

  for (const item of sequence) {
    if (typeof item === "number") {
      await new Promise((resolve) => setTimeout(resolve, item));
    } else {
      playHapticFeedback(item);
    }
  }
}
