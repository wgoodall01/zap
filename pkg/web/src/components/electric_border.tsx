import React, {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useCallback,
} from "react";
import "./electric_border.css";

// Adapted from:
// https://reactbits.dev/animations/electric-border

export const ElectricBorder = ({
  children,
  color = "var(--accent-9)",
  speed = 1,
  chaos = 1,
  thickness = 2,
  borderGlow = true,
  backgroundGlow = true,
  className,
  style,
}: {
  children: React.ReactNode;
  color?: string; // CSS color for the electric effect
  speed?: number; // Speed multiplier for the animation
  chaos?: number; // Chaos multiplier for displacement intensity
  thickness?: number; // Border thickness in pixels
  borderGlow?: boolean; // Whether to show the border glow effect
  backgroundGlow?: boolean; // Whether to show the background glow effect
  className?: string; // Additional CSS classes for the root element
  style?: React.CSSProperties; // Inline styles for the root element
}) => {
  const rawId = useId().replace(/[:]/g, "");
  const filterId = `turbulent-displace-${rawId}`;

  // Refs for component elements
  const rootRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const strokeRef = useRef<HTMLDivElement>(null);

  // Refs to cache query results for SVG filter elements
  const dyAnimsRef = useRef<SVGAnimateElement[]>([]);
  const dxAnimsRef = useRef<SVGAnimateElement[]>([]);
  const displacementMapRef = useRef<SVGFEDisplacementMapElement | null>(null);

  // --- Animation Restart Logic ---
  // Memoized function to restart the SVG animations.
  // This is necessary when duration or size-based values change.
  const restartAnimations = useCallback(() => {
    const allAnims = [...dyAnimsRef.current, ...dxAnimsRef.current];
    // requestAnimationFrame ensures 'beginElement' runs after DOM updates.
    requestAnimationFrame(() => {
      allAnims.forEach((anim) => {
        if (typeof anim.beginElement === "function") {
          try {
            anim.beginElement();
          } catch (e) {
            console.warn("ElectricBorder: beginElement failed.", e);
          }
        }
      });
    });
  }, []);

  // --- Effect 1: Initialization and Element Caching ---
  // Runs once on mount to find and store relevant SVG elements and set initial filter properties.
  useEffect(() => {
    const svg = svgRef.current;
    if (!svg || !strokeRef.current) return;

    // Apply filter to the target element
    strokeRef.current.style.filter = `url(#${filterId})`;

    // Cache element references to avoid re-querying in other effects
    dyAnimsRef.current = Array.from(
      svg.querySelectorAll('feOffset > animate[attributeName="dy"]'),
    );
    dxAnimsRef.current = Array.from(
      svg.querySelectorAll('feOffset > animate[attributeName="dx"]'),
    );
    displacementMapRef.current = svg.querySelector("feDisplacementMap");

    // Set static filter attributes once
    const filterEl = svg.querySelector(`#${CSS.escape(filterId)}`);
    if (filterEl) {
      filterEl.setAttribute("x", "-200%");
      filterEl.setAttribute("y", "-200%");
      filterEl.setAttribute("width", "500%");
      filterEl.setAttribute("height", "500%");
    }
  }, [filterId]); // Re-run if filterId changes (e.g., in strict mode double-render)

  // --- Effect 2: Resize Handling ---
  // Updates animation values based on component dimensions.
  useLayoutEffect(() => {
    const host = rootRef.current;
    if (!host) return;

    const updateSizeDependentAttributes = () => {
      const width = Math.max(1, Math.round(host.clientWidth || 0));
      const height = Math.max(1, Math.round(host.clientHeight || 0));

      if (dyAnimsRef.current.length >= 2) {
        dyAnimsRef.current[0]?.setAttribute("values", `${height}; 0`);
        dyAnimsRef.current[1]?.setAttribute("values", `0; -${height}`);
      }
      if (dxAnimsRef.current.length >= 2) {
        dxAnimsRef.current[0]?.setAttribute("values", `${width}; 0`);
        dxAnimsRef.current[1]?.setAttribute("values", `0; -${width}`);
      }
      restartAnimations();
    };

    const resizeObserver = new ResizeObserver(updateSizeDependentAttributes);
    resizeObserver.observe(host);
    updateSizeDependentAttributes(); // Initial call to set size

    return () => resizeObserver.disconnect();
  }, [restartAnimations]);

  // --- Effect 3: Speed Prop Handling ---
  // Updates animation duration when the 'speed' prop changes.
  useEffect(() => {
    const baseDuration = 6;
    const duration = Math.max(0.001, baseDuration / (speed || 1));
    const allAnims = [...dyAnimsRef.current, ...dxAnimsRef.current];

    allAnims.forEach((anim) => anim.setAttribute("dur", `${duration}s`));
    restartAnimations();
  }, [speed, restartAnimations]);

  // --- Effect 4: Chaos Prop Handling ---
  // Updates displacement map scale when the 'chaos' prop changes.
  useEffect(() => {
    if (displacementMapRef.current) {
      displacementMapRef.current.setAttribute(
        "scale",
        String(30 * (chaos || 1)),
      );
    }
  }, [chaos]);

  // CSS variables for non-SVG properties like color and thickness
  const vars = {
    ["--electric-border-color"]: color,
    ["--eb-border-width"]: `${thickness}px`,
  };

  return (
    <div
      ref={rootRef}
      className={`electric-border ${className ?? ""}`}
      style={{ ...vars, ...style }}
    >
      <svg ref={svgRef} className="eb-svg" aria-hidden focusable="false">
        <defs>
          <filter id={filterId} colorInterpolationFilters="sRGB">
            {/* --- SVG filter definition remains unchanged --- */}
            <feTurbulence
              type="turbulence"
              baseFrequency="0.02"
              numOctaves="10"
              result="noise1"
              seed="1"
            />
            <feOffset in="noise1" dx="0" dy="0" result="offsetNoise1">
              <animate
                attributeName="dy"
                values="700; 0"
                dur="6s"
                repeatCount="indefinite"
                calcMode="linear"
              />
            </feOffset>
            <feTurbulence
              type="turbulence"
              baseFrequency="0.02"
              numOctaves="10"
              result="noise2"
              seed="1"
            />
            <feOffset in="noise2" dx="0" dy="0" result="offsetNoise2">
              <animate
                attributeName="dy"
                values="0; -700"
                dur="6s"
                repeatCount="indefinite"
                calcMode="linear"
              />
            </feOffset>
            <feTurbulence
              type="turbulence"
              baseFrequency="0.02"
              numOctaves="10"
              result="noise1"
              seed="2"
            />
            <feOffset in="noise1" dx="0" dy="0" result="offsetNoise3">
              <animate
                attributeName="dx"
                values="490; 0"
                dur="6s"
                repeatCount="indefinite"
                calcMode="linear"
              />
            </feOffset>
            <feTurbulence
              type="turbulence"
              baseFrequency="0.02"
              numOctaves="10"
              result="noise2"
              seed="2"
            />
            <feOffset in="noise2" dx="0" dy="0" result="offsetNoise4">
              <animate
                attributeName="dx"
                values="0; -490"
                dur="6s"
                repeatCount="indefinite"
                calcMode="linear"
              />
            </feOffset>
            <feComposite in="offsetNoise1" in2="offsetNoise2" result="part1" />
            <feComposite in="offsetNoise3" in2="offsetNoise4" result="part2" />
            <feBlend
              in="part1"
              in2="part2"
              mode="color-dodge"
              result="combinedNoise"
            />
            <feDisplacementMap
              in="SourceGraphic"
              in2="combinedNoise"
              scale="30"
              xChannelSelector="R"
              yChannelSelector="B"
            />
          </filter>
        </defs>
      </svg>

      <div className="eb-layers">
        <div ref={strokeRef} className="eb-stroke" />

        {borderGlow && (
          <>
            <div className="eb-glow-1" />
            <div className="eb-glow-2" />
          </>
        )}
        {backgroundGlow && <div className="eb-background-glow" />}
      </div>

      <div className="eb-content">{children}</div>
    </div>
  );
};
