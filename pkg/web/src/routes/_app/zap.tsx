import { useState, useEffect } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Button, Flex } from "@radix-ui/themes";
import { Lightning } from "../../components/lightning";
import { ElectricBorder } from "../../components/electric_border";
import { LightningIcon } from "@phosphor-icons/react";
import { type TgHaptic, playHapticFeedback } from "../../telegram";

export const Route = createFileRoute("/_app/zap")({
  component: ZapPage,
  head: () => ({
    meta: [{ title: "ZAP ZAP" }],
  }),
});

// Duration of the effect, random distribution:
const EFFECT_DURATION_MEAN = 700;
const EFFECT_DURATION_STD = 150;
function effectDuration() {
  return Math.max(
    100,
    Math.round(
      EFFECT_DURATION_MEAN +
        EFFECT_DURATION_STD * (Math.random() * 2 - 1) +
        (Math.random() * 2 - 1) * EFFECT_DURATION_STD,
    ),
  );
}

function ZapPage() {
  // State to trigger the lightning effect
  const [firing, setFiring] = useState(false);
  useEffect(() => {
    if (firing) {
      playZapHaptics();

      const timeout = setTimeout(() => {
        setFiring(false);
      }, effectDuration());
      return () => clearTimeout(timeout);
    }
  }, [firing]);

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
        <ElectricBorder
          style={{ borderRadius: "100%" }}
          speed={3}
          chaos={0.7}
          thickness={5}
          borderGlow={true}
          backgroundGlow={true}
        >
          <Button
            onClick={() => setFiring(true)}
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
      </Flex>
    </Flex>
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
