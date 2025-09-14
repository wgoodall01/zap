import React from "react";

interface ContainerProps {
  r: string;
  gap: string;
  children: React.ReactNode;
}

interface CenteredProps {
  children: React.ReactNode;
}

interface AroundProps {
  theta: number;
  orient?: boolean;
  children: React.ReactNode;
}

const Container: React.FC<ContainerProps> = ({ r, gap, children }) => {
  const containerStyle: React.CSSProperties = {
    position: "relative",
    width: `calc(2 * ${r})`,
    height: `calc(2 * ${r})`,
    "--radial-radius": r,
    "--radial-gap": gap,
  } as React.CSSProperties;

  return <div style={containerStyle}>{children}</div>;
};

const Centered: React.FC<CenteredProps> = ({ children }) => {
  const centeredStyle: React.CSSProperties = {
    position: "absolute",
    top: 0,
    left: 0,
    width: "100%",
    height: "100%",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  };

  return <div style={centeredStyle}>{children}</div>;
};

const Around: React.FC<AroundProps> = ({ theta, children, orient = false }) => {
  const aroundStyle: React.CSSProperties = {
    position: "absolute",
    top: "50%",
    left: "50%",
    "--angle": `${theta}rad`,
    "--radius": `calc(var(--radial-radius) + var(--radial-gap))`,
    translate:
      "calc(cos(var(--angle)) * var(--radius)) calc(sin(var(--angle)) * var(--radius))",
    transform: `translate(-50%, -50%) rotate(${orient ? theta + Math.PI / 2 : 0}rad)`,
  } as React.CSSProperties;

  return <div style={aroundStyle}>{children}</div>;
};

export const RadialLayout = {
  Container,
  Centered,
  Around,
};
