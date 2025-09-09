import { useState, useEffect, useCallback } from "react";
import { createFileRoute } from "@tanstack/react-router";
import {
  Button,
  Flex,
  Dialog,
  Badge,
  Switch,
  Text,
  Box,
  ScrollArea,
  Card,
} from "@radix-ui/themes";
import { Lightning } from "../../components/lightning";
import { ElectricBorder } from "../../components/electric_border";
import {
  LightningIcon,
  GearIcon,
  Vibrate,
  SpeakerHigh,
} from "@phosphor-icons/react";
import { type TgHaptic, playHapticFeedback } from "../../telegram";
import { $api } from "../../api";
import {
  mutationDevicesTrigger,
  queryDevicesList,
} from "../../api_client/@tanstack/react-query.gen";
import type {
  TriggerRequest,
  DeviceWithShockers,
  Shocker,
} from "../../api_client/types.gen";

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
    queryDevicesList,
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
  const shockMutation = $api.useMutation(mutationDevicesTrigger);

  // Event handler: trigger shock
  const onShock = useCallback(async () => {
    if (firing) return; // Prevent multiple triggers

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
            duration: 1000, // 1 second
          },
        },
      },
    });
  }, [firing, shockMutation]);

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
        {selectedShockers.size === 0 ? (
          <Card style={{ maxWidth: 300, textAlign: "center" }}>
            <Text size="3" mb="2">
              No shockers enabled.
            </Text>
            <Button variant="soft" size="2" onClick={() => setModalOpen(true)}>
              <GearIcon />
              Open Settings
            </Button>
          </Card>
        ) : (
          <ElectricBorder
            style={{ borderRadius: "100%" }}
            speed={3}
            chaos={0.7}
            thickness={5}
            borderGlow={true}
            backgroundGlow={true}
          >
            <Button
              onClick={onShock}
              size="4"
              variant="soft"
              style={{
                fontSize: "7rem",
                width: "15rem",
                height: "15rem",
                appearance: "none",
                boxShadow: "var(--shadow-5)",
                borderRadius: "100%",
                backdropFilter: "blur(15px)",
              }}
            >
              <LightningIcon weight={firing ? "fill" : "light"} />
            </Button>
          </ElectricBorder>
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
          isOpen={modalOpen}
          onOpenChange={setModalOpen}
        />
      </Flex>
    </Flex>
  );
}

interface ShockersDialogProps {
  devices: DeviceWithShockers[];
  enabledShockers: string[];
  onChangeEnabled: (shockerIds: string[]) => void;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

function ShockersDialog({
  devices,
  enabledShockers,
  onChangeEnabled,
  isOpen,
  onOpenChange,
}: ShockersDialogProps) {
  const enabledSet = new Set(enabledShockers);

  return (
    <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <Dialog.Trigger>
        <Button variant="soft" size="3">
          <GearIcon />
          Shockers <Badge size="2">{enabledShockers.length}</Badge>
        </Button>
      </Dialog.Trigger>
      <Dialog.Content style={{ maxWidth: 450 }}>
        <Dialog.Title>Shockers</Dialog.Title>
        <Dialog.Description size="2" mb="4">
          Choose which shockers to activate when you press the zap button.
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
                        {shocker.isPaused && " (Paused)"}
                      </Text>
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
